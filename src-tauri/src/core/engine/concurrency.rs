use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BusyState {
    None,
    Text,
    WordAlternatives,
    Subtitle,
}

struct CoordinatorState {
    busy: BusyState,
    active_lease_id: u64,
    next_lease_id: u64,
    text_cancel: Option<CancellationToken>,
}

/// Exclusive coordinator protecting `--parallel 1` on `llama-server`.
pub struct InferenceCoordinator {
    state: Arc<Mutex<CoordinatorState>>,
    idle_notify: Arc<Notify>,
}

pub struct InferenceGuard {
    lease_id: u64,
    state: Arc<Mutex<CoordinatorState>>,
    idle_notify: Arc<Notify>,
}

impl Drop for InferenceGuard {
    fn drop(&mut self) {
        let notify = Arc::clone(&self.idle_notify);
        let lease_id = self.lease_id;

        let released = {
            if let Ok(mut guard) = self.state.try_lock() {
                if guard.active_lease_id == lease_id {
                    guard.busy = BusyState::None;
                    guard.text_cancel = None;
                    notify.notify_waiters();
                }
                true
            } else {
                false
            }
        };

        if !released {
            let state = Arc::clone(&self.state);
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let mut guard = state.lock().await;
                    if guard.active_lease_id == lease_id {
                        guard.busy = BusyState::None;
                        guard.text_cancel = None;
                        notify.notify_waiters();
                    }
                });
            }
        }
    }
}

impl Default for InferenceCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl InferenceCoordinator {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(CoordinatorState {
                busy: BusyState::None,
                active_lease_id: 0,
                next_lease_id: 1,
                text_cancel: None,
            })),
            idle_notify: Arc::new(Notify::new()),
        }
    }

    pub async fn current_busy(&self) -> BusyState {
        self.state.lock().await.busy
    }

    pub fn current_busy_nonblocking(&self) -> BusyState {
        self.state
            .try_lock()
            .map(|g| g.busy)
            .unwrap_or(BusyState::None)
    }

    pub async fn cancel_text_stream(&self) {
        let token = {
            let guard = self.state.lock().await;
            if guard.busy == BusyState::Text {
                guard.text_cancel.clone()
            } else {
                None
            }
        };
        if let Some(t) = token {
            t.cancel();
        }
    }

    pub async fn acquire_text_stream(&self) -> AppResult<(InferenceGuard, CancellationToken)> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);

        loop {
            let notified = self.idle_notify.notified();
            let cancel_to_trigger = {
                let mut guard = self.state.lock().await;
                match guard.busy {
                    BusyState::None => {
                        let lease_id = guard.next_lease_id;
                        guard.next_lease_id += 1;
                        guard.active_lease_id = lease_id;
                        guard.busy = BusyState::Text;
                        let token = CancellationToken::new();
                        guard.text_cancel = Some(token.clone());
                        return Ok((
                            InferenceGuard {
                                lease_id,
                                state: Arc::clone(&self.state),
                                idle_notify: Arc::clone(&self.idle_notify),
                            },
                            token,
                        ));
                    }
                    BusyState::Subtitle => return Err(AppError::EngineBusySubtitleJob),
                    BusyState::Text => guard.text_cancel.clone(),
                    BusyState::WordAlternatives => None,
                }
            };

            if let Some(token) = cancel_to_trigger {
                token.cancel();
            }

            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                return Err(AppError::InvalidInput(
                    "Previous translation request is still cancelling".to_string(),
                ));
            }
        }
    }

    pub async fn acquire_word_alternatives(&self) -> AppResult<InferenceGuard> {
        let mut guard = self.state.lock().await;
        match guard.busy {
            BusyState::None => {
                let lease_id = guard.next_lease_id;
                guard.next_lease_id += 1;
                guard.active_lease_id = lease_id;
                guard.busy = BusyState::WordAlternatives;
                guard.text_cancel = None;
                Ok(InferenceGuard {
                    lease_id,
                    state: Arc::clone(&self.state),
                    idle_notify: Arc::clone(&self.idle_notify),
                })
            }
            BusyState::Subtitle => Err(AppError::EngineBusySubtitleJob),
            BusyState::Text | BusyState::WordAlternatives => Err(AppError::InvalidInput(
                "Wait for text translation streaming to finish before opening word alternatives"
                    .to_string(),
            )),
        }
    }

    pub async fn acquire_subtitle_job(&self) -> AppResult<InferenceGuard> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);

        loop {
            let notified = self.idle_notify.notified();
            let cancel_to_trigger = {
                let mut guard = self.state.lock().await;
                match guard.busy {
                    BusyState::None => {
                        let lease_id = guard.next_lease_id;
                        guard.next_lease_id += 1;
                        guard.active_lease_id = lease_id;
                        guard.busy = BusyState::Subtitle;
                        guard.text_cancel = None;
                        return Ok(InferenceGuard {
                            lease_id,
                            state: Arc::clone(&self.state),
                            idle_notify: Arc::clone(&self.idle_notify),
                        });
                    }
                    BusyState::Subtitle => return Err(AppError::EngineBusySubtitleJob),
                    BusyState::Text => guard.text_cancel.clone(),
                    BusyState::WordAlternatives => None,
                }
            };

            if let Some(token) = cancel_to_trigger {
                token.cancel();
            }

            if tokio::time::timeout_at(deadline, notified).await.is_err() {
                return Err(AppError::InvalidInput(
                    "AI engine is busy processing another request".to_string(),
                ));
            }
        }
    }

    /// Ensures no subtitle job is running before model switching/deletion or settings update.
    pub async fn ensure_idle_for_admin_action(&self) -> AppResult<()> {
        let guard = self.state.lock().await;
        match guard.busy {
            BusyState::Subtitle => Err(AppError::EngineBusySubtitleJob),
            BusyState::Text | BusyState::WordAlternatives => Err(AppError::InvalidInput(
                "Wait for translation to finish before switching models or engine settings"
                    .to_string(),
            )),
            BusyState::None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subtitle_lock_blocks_text_alternatives_and_admin_actions() {
        let coord = InferenceCoordinator::new();
        let sub_guard = coord.acquire_subtitle_job().await.expect("acquire subtitle");
        assert_eq!(coord.current_busy().await, BusyState::Subtitle);

        assert!(matches!(
            coord.acquire_text_stream().await,
            Err(AppError::EngineBusySubtitleJob)
        ));
        assert!(matches!(
            coord.acquire_word_alternatives().await,
            Err(AppError::EngineBusySubtitleJob)
        ));
        assert!(matches!(
            coord.ensure_idle_for_admin_action().await,
            Err(AppError::EngineBusySubtitleJob)
        ));
        assert!(matches!(
            coord.acquire_subtitle_job().await,
            Err(AppError::EngineBusySubtitleJob)
        ));

        drop(sub_guard);
        assert_eq!(coord.current_busy().await, BusyState::None);
        assert!(coord.ensure_idle_for_admin_action().await.is_ok());
    }

    #[tokio::test]
    async fn text_stream_cancels_previous_stream_before_reacquiring() {
        let coord = Arc::new(InferenceCoordinator::new());
        let (guard1, token1) = coord.acquire_text_stream().await.expect("first stream");
        assert!(!token1.is_cancelled());

        let coord_clone = Arc::clone(&coord);
        let t1_clone = token1.clone();
        let handle = tokio::spawn(async move {
            t1_clone.cancelled().await;
            drop(guard1);
        });

        let (guard2, token2) = coord_clone
            .acquire_text_stream()
            .await
            .expect("second stream replaces first");
        assert!(token1.is_cancelled());
        assert!(!token2.is_cancelled());
        handle.await.unwrap();
        drop(guard2);
        assert_eq!(coord.current_busy().await, BusyState::None);
    }
}
