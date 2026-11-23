use crate::core::engine::sanitizer::{strip_think_tags, StreamingThinkSanitizer};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChatCompletionRequest {
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub response_format: Option<serde_json::Value>,
    pub disable_thinking: bool,
}

#[derive(Debug, Clone)]
pub struct ChatCompletionResponse {
    pub content: String,
    pub ttft_ms: u64,
    pub total_tokens: u32,
    pub tok_per_sec: f64,
}

#[derive(Debug, Clone)]
pub struct StreamMetrics {
    pub full_text: String,
    pub ttft_ms: u64,
    pub total_tokens: u32,
    pub tok_per_sec: f64,
    pub cancelled: bool,
}

#[derive(Clone)]
pub struct LlamaClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl LlamaClient {
    pub fn new(port: u16, api_key: String) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .build()?;

        Ok(Self {
            http,
            base_url: format!("http://127.0.0.1:{port}"),
            api_key,
        })
    }

    #[cfg(test)]
    pub fn from_base_url(base_url: String, api_key: String) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self {
            http,
            base_url,
            api_key,
        })
    }

    pub async fn health_check(&self) -> AppResult<bool> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.api_key)
            .timeout(Duration::from_secs(2))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    pub async fn chat_completion_json(
        &self,
        req: ChatCompletionRequest,
    ) -> AppResult<ChatCompletionResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let payload = build_payload(&req, false);
        let start = Instant::now();

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await?;
        if !status.is_success() {
            return Err(classify_http_error(status.as_u16(), &body_text));
        }

        let parsed: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
            AppError::HttpError(format!("Invalid llama-server JSON response: {e}"))
        })?;

        let raw_content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                AppError::HttpError("Empty content field in llama-server response".to_string())
            })?;
        let content = strip_think_tags(raw_content);

        let elapsed = start.elapsed();
        let ttft_ms = elapsed.as_millis() as u64;
        let total_tokens = parsed["usage"]["completion_tokens"]
            .as_u64()
            .map(|n| n as u32)
            .unwrap_or_else(|| content.split_whitespace().count().max(1) as u32);
        let secs = elapsed.as_secs_f64().max(0.001);
        let tok_per_sec = f64::from(total_tokens) / secs;

        Ok(ChatCompletionResponse {
            content,
            ttft_ms,
            total_tokens,
            tok_per_sec,
        })
    }

    pub async fn chat_completion_stream<F>(
        &self,
        req: ChatCompletionRequest,
        cancel: CancellationToken,
        mut on_delta: F,
    ) -> AppResult<StreamMetrics>
    where
        F: FnMut(&str),
    {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let payload = build_payload(&req, true);
        let start = Instant::now();

        let mut resp = tokio::select! {
            _ = cancel.cancelled() => {
                return Ok(StreamMetrics {
                    full_text: String::new(),
                    ttft_ms: 0,
                    total_tokens: 0,
                    tok_per_sec: 0.0,
                    cancelled: true,
                });
            }
            res = self.http.post(&url).bearer_auth(&self.api_key).json(&payload).send() => {
                res?
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(classify_http_error(status.as_u16(), &err_body));
        }

        let mut sanitizer = StreamingThinkSanitizer::new();
        let mut full_text = String::new();
        let mut line_buf = String::new();
        let mut ttft_ms: Option<u64> = None;
        let mut chunk_token_count: u32 = 0;
        let mut usage_tokens: Option<u32> = None;
        let mut cancelled = false;

        loop {
            let next_chunk = tokio::select! {
                _ = cancel.cancelled() => {
                    cancelled = true;
                    break;
                }
                chunk = resp.chunk() => chunk?,
            };

            let Some(bytes) = next_chunk else {
                break;
            };

            let text = String::from_utf8_lossy(&bytes);
            line_buf.push_str(&text);

            while let Some(newline_pos) = line_buf.find('\n') {
                let raw_line = line_buf[..newline_pos].trim_end_matches('\r').to_string();
                line_buf.drain(..=newline_pos);

                if process_sse_line(
                    &raw_line,
                    &mut sanitizer,
                    &mut full_text,
                    &mut ttft_ms,
                    &mut chunk_token_count,
                    &mut usage_tokens,
                    start,
                    &mut on_delta,
                ) {
                    break;
                }
            }
        }

        if !cancelled && !line_buf.trim().is_empty() {
            let remaining = line_buf.trim().to_string();
            let _ = process_sse_line(
                &remaining,
                &mut sanitizer,
                &mut full_text,
                &mut ttft_ms,
                &mut chunk_token_count,
                &mut usage_tokens,
                start,
                &mut on_delta,
            );
        }

        if !cancelled {
            let tail = sanitizer.finish();
            if !tail.is_empty() {
                if ttft_ms.is_none() {
                    ttft_ms = Some(start.elapsed().as_millis() as u64);
                }
                full_text.push_str(&tail);
                on_delta(&tail);
            }
        }

        let elapsed_secs = start.elapsed().as_secs_f64().max(0.001);
        let total_tokens = usage_tokens.unwrap_or(chunk_token_count);
        let tok_per_sec = if total_tokens > 0 {
            f64::from(total_tokens) / elapsed_secs
        } else {
            0.0
        };

        Ok(StreamMetrics {
            full_text,
            ttft_ms: ttft_ms.unwrap_or(0),
            total_tokens,
            tok_per_sec,
            cancelled,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn process_sse_line<F: FnMut(&str)>(
    line: &str,
    sanitizer: &mut StreamingThinkSanitizer,
    full_text: &mut String,
    ttft_ms: &mut Option<u64>,
    chunk_token_count: &mut u32,
    usage_tokens: &mut Option<u32>,
    start: Instant,
    on_delta: &mut F,
) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with(':') {
        return false;
    }
    let Some(data) = trimmed.strip_prefix("data:") else {
        return false;
    };
    let payload = data.trim();
    if payload == "[DONE]" {
        return true;
    }

    let Ok(val) = serde_json::from_str::<serde_json::Value>(payload) else {
        return false;
    };

    if let Some(ct) = val["usage"]["completion_tokens"].as_u64() {
        *usage_tokens = Some(ct as u32);
    }

    if let Some(delta) = val["choices"][0]["delta"]["content"].as_str() {
        if !delta.is_empty() {
            *chunk_token_count += 1;
            let clean = sanitizer.push(delta);
            if !clean.is_empty() {
                if ttft_ms.is_none() {
                    *ttft_ms = Some(start.elapsed().as_millis() as u64);
                }
                full_text.push_str(&clean);
                on_delta(&clean);
            }
        }
    }

    false
}

fn build_payload(req: &ChatCompletionRequest, stream: bool) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "messages": req.messages,
        "temperature": req.temperature,
        "stream": stream,
    });

    if let Some(max_tok) = req.max_tokens {
        obj["max_tokens"] = serde_json::json!(max_tok);
    }
    if let Some(fmt) = &req.response_format {
        obj["response_format"] = fmt.clone();
    }
    if req.disable_thinking {
        obj["chat_template_kwargs"] = serde_json::json!({ "enable_thinking": false });
    }
    if stream {
        obj["stream_options"] = serde_json::json!({ "include_usage": true });
    }

    obj
}

fn classify_http_error(status: u16, body: &str) -> AppError {
    let lower = body.to_lowercase();
    if lower.contains("out of memory")
        || lower.contains("vk_error_out_of_device_memory")
        || lower.contains("failed to allocate")
    {
        return AppError::OutOfMemory;
    }
    AppError::HttpError(format!("llama-server HTTP {status}: {body}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn spawn_mock_sse_server(expected_token: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut req_buf = Vec::new();
                    let mut tmp = [0u8; 2048];
                    loop {
                        let n = stream.read(&mut tmp).await.unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        req_buf.extend_from_slice(&tmp[..n]);
                        let text = String::from_utf8_lossy(&req_buf);
                        if let Some(hdr_end) = text.find("\r\n\r\n") {
                            let headers = &text[..hdr_end];
                            let content_len = headers
                                .lines()
                                .find_map(|line| {
                                    let (k, v) = line.split_once(':')?;
                                    if k.eq_ignore_ascii_case("content-length") {
                                        v.trim().parse::<usize>().ok()
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(0);
                            if req_buf.len() >= hdr_end + 4 + content_len {
                                break;
                            }
                        }
                    }
                    let req_text = String::from_utf8_lossy(&req_buf);
                    let expected_header = format!("authorization: bearer {}", expected_token);
                    if !req_text.to_lowercase().contains(&expected_header) {
                        let _ = stream
                            .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n")
                            .await;
                        return;
                    }

                    let body = concat!(
                        "data: {\"choices\":[{\"delta\":{\"content\":\"<think>ok\"}}]}\n\n",
                        "data: {\"choices\":[{\"delta\":{\"content\":\"</think>Halo \"}}]}\n\n",
                        "data: {\"choices\":[{\"delta\":{\"content\":\"dunia\"}}]}\n\n",
                        "data: {\"choices\":[],\"usage\":{\"completion_tokens\":5}}\n\n",
                        "data: [DONE]\n\n"
                    );
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                });
            }
        });

        format!("http://{addr}")
    }

    #[tokio::test]
    async fn llama_client_streams_with_bearer_and_sanitizes_think() {
        let base_url = spawn_mock_sse_server("secret_test_key").await;
        let client = LlamaClient::from_base_url(base_url, "secret_test_key".to_string()).unwrap();

        let req = ChatCompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hello world".to_string(),
            }],
            temperature: 0.2,
            max_tokens: Some(128),
            response_format: None,
            disable_thinking: true,
        };

        let mut chunks = Vec::new();
        let metrics = client
            .chat_completion_stream(req, CancellationToken::new(), |delta| {
                chunks.push(delta.to_string());
            })
            .await
            .unwrap_or_else(|e| panic!("SSE stream failed: {e:?}"));

        assert!(!metrics.cancelled);
        assert_eq!(metrics.full_text, "Halo dunia");
        assert_eq!(chunks.concat(), "Halo dunia");
        assert_eq!(metrics.total_tokens, 5);
        assert!(metrics.tok_per_sec > 0.0);
    }

    #[tokio::test]
    async fn llama_client_respects_cancellation_token() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf).await;
                let _ = stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
                    .await;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        let client = LlamaClient::from_base_url(format!("http://{addr}"), "k".to_string()).unwrap();
        let req = ChatCompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "Hi".to_string(),
            }],
            temperature: 0.2,
            max_tokens: None,
            response_format: None,
            disable_thinking: true,
        };

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            cancel_clone.cancel();
        });

        let metrics = client
            .chat_completion_stream(req, cancel, |_| {})
            .await
            .expect("cancelled stream returns cleanly");
        assert!(metrics.cancelled);
    }
}
