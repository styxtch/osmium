import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface AppErrorPayload {
  code: string;
  message: string;
}

export type HardwareTier = "optimal_8b" | "balanced_4b" | "fallback_cpu";
export type SidecarStatus = "unconfigured" | "standby" | "loading" | "online" | "error";
export type BusyState = "none" | "text" | "word_alternatives" | "subtitle";

export interface HardwareProfile {
  gpu_name: string;
  dedicated_vram_mb: number;
  shared_memory_mb: number;
  current_vram_usage_mb: number;
  system_ram_gb: number;
  vulkan_supported: boolean;
  vulkan_version: string | null;
  vulkan_device_id: string | null;
  flash_attention_supported: boolean;
  recommended_tier: HardwareTier;
  thermal_c: number | null;
  clock_mhz: number | null;
}

export interface EngineTelemetry {
  status: SidecarStatus;
  pid: number | null;
  port: number | null;
  active_model_id: string | null;
  active_model_name: string | null;
  backend: "vulkan" | "cpu" | string;
  ctx_size: number;
  gpu_layers: number;
  vram_used_mb: number;
  vram_total_mb: number;
  last_ttft_ms: number | null;
  last_tok_per_sec: number | null;
  db_wal_synced: boolean;
  busy_by: BusyState;
}

export interface VramTickPayload {
  vram_used_mb: number;
  vram_total_mb: number;
  gpu_temp_c?: number | null;
  gpu_clock_mhz?: number | null;
  ram_used_gb?: number;
  ram_total_gb?: number;
}

export interface DiskSpaceInfo {
  free_bytes: number;
  writable: boolean;
}

export interface AppSettings {
  display_mode: "standard" | "compact";
  default_model_dir: string;
  default_export_dir: string;
  backend_override: "auto" | "vulkan" | "cpu";
  ctx_size_override: "auto" | "2048" | "4096" | "8192";
  gpu_layers_override: string;
  tts_provider: "native" | "neural" | string;
  tts_rate: number;
  tts_voice_prefs_json: string;
  auto_idle_unload_minutes: number;
}

export interface CatalogModel {
  id: string;
  display_name: string;
  tier: string;
  quantization: string;
  repo_id: string;
  commit_sha: string;
  file_name: string;
  download_url: string;
  file_size_bytes: number;
  estimated_vram_mb: number;
  sha256: string;
  block_count: number;
  description: string;
  badge_label: string;
}

export interface InstalledModelWithHealth {
  id: string;
  display_name: string;
  tier: string;
  file_path: string;
  file_size_bytes: number;
  sha256: string | null;
  quantization: string | null;
  is_byom: boolean;
  block_count: number | null;
  is_active: boolean;
  created_at: number;
  file_exists: boolean;
}

export interface DownloadTask {
  model_id: string;
  download_url: string;
  target_path: string;
  part_path: string;
  expected_sha256: string;
  etag: string | null;
  total_bytes: number;
  bytes_downloaded: number;
  status: "downloading" | "paused" | "verifying" | "failed" | "completed" | "cancelled" | string;
  error_msg: string | null;
  updated_at: number;
}

export interface DownloadProgressEvent {
  model_id: string;
  bytes_downloaded: number;
  total_bytes: number;
  percent: number;
  speed_mb_per_sec: number;
  eta_seconds: number | null;
  status: string;
  error_msg: string | null;
}

export interface ModelsOverview {
  catalog: CatalogModel[];
  installed: InstalledModelWithHealth[];
  downloads: DownloadTask[];
}

export type Tone = "natural" | "formal" | "casual";

export interface TranslationStreamEvent {
  event_type: "status" | "chunk" | "done" | "cancelled" | string;
  delta: string | null;
  chunk_index: number;
  total_chunks: number;
  ttft_ms: number | null;
  tok_per_sec: number | null;
  total_tokens: number | null;
  status_message: string | null;
}

export interface TranslationResult {
  translated_text: string;
  source_lang: string;
  target_lang: string;
  tone: Tone;
  model_id: string;
  model_display_name: string;
  ttft_ms: number;
  tok_per_sec: number;
  total_tokens: number;
  chunk_count: number;
  cancelled: boolean;
}

export interface WordAlternativeItem {
  word: string;
  nuance: string;
  recommended: boolean;
}

export interface WordAlternativesResponse {
  context_summary: string;
  alternatives: WordAlternativeItem[];
}

export interface TextHistoryItem {
  id: string;
  source_text: string;
  translated_text: string;
  source_lang: string;
  target_lang: string;
  tone: Tone;
  model_id: string;
  model_display_name: string;
  ttft_ms: number | null;
  tok_per_sec: number | null;
  created_at: number;
}

export interface Glossary {
  id: string;
  name: string;
  entry_count: number;
  created_at: number;
  updated_at: number;
}

export interface GlossaryEntry {
  id: string;
  glossary_id: string;
  source_term: string;
  target_term: string;
  case_sensitive: boolean;
}

export interface GlossaryImportResult {
  glossary: Glossary;
  imported_count: number;
  skipped_count: number;
}

export type SubtitleGenre =
  | "general"
  | "anime"
  | "film"
  | "documentary"
  | "gaming";

export type SubtitleJobStatus =
  | "idle"
  | "running"
  | "paused"
  | "completed"
  | "cancelled"
  | "error";

export type SubtitleCueStatus =
  | "queued"
  | "processing"
  | "done"
  | "warning"
  | "skipped";

export type SubtitleCueFilter = "all" | SubtitleCueStatus;

export interface SubtitleCueMeta {
  srt_sequence?: string;
  vtt_identifier?: string;
  vtt_settings?: string;
  ass_raw_prefix?: string;
  leading_tags?: string;
  trailing_tags?: string;
  display_tags?: string[];
}

export interface GlossaryMatch {
  source_term: string;
  target_term: string;
  case_sensitive: boolean;
}

export interface SubtitleJobSummary {
  id: string;
  file_name: string;
  file_path: string;
  file_size_bytes: number;
  format: "srt" | "vtt" | "ass" | string;
  source_lang: string;
  target_lang: string;
  genre: SubtitleGenre | string;
  glossary_id: string | null;
  tag_shield_enabled: boolean;
  model_id: string;
  model_display_name: string;
  status: SubtitleJobStatus;
  total_cues: number;
  completed_cues: number;
  self_healing_errors: number;
  created_at: number;
  updated_at: number;
}

export interface SubtitleCueDto {
  job_id: string;
  cue_index: number;
  cue_type: "dialogue" | "comment" | "drawing" | string;
  start_time: string;
  end_time: string;
  meta_json: string | null;
  glossary_matches_json: string | null;
  original_text: string;
  translated_text: string | null;
  batch_number: number | null;
  status: SubtitleCueStatus;
  warning_msg: string | null;
}

export interface SubtitleBatchStartedEvent {
  job_id: string;
  batch_number: number;
  cue_indices: number[];
}

export interface SubtitleBatchCompletedEvent {
  job_id: string;
  batch_number: number;
  cues: SubtitleCueDto[];
  completed_cues: number;
  total_cues: number;
  self_healing_errors: number;
  status: SubtitleJobStatus;
  tok_per_sec: number;
}

export interface SubtitleJobErrorEvent {
  job_id: string;
  code: string;
  message: string;
}

export function parseCueMeta(metaJson: string | null): SubtitleCueMeta {
  if (!metaJson) return {};
  try {
    return JSON.parse(metaJson) as SubtitleCueMeta;
  } catch {
    return {};
  }
}

export function parseGlossaryMatches(
  matchesJson: string | null
): GlossaryMatch[] {
  if (!matchesJson) return [];
  try {
    return JSON.parse(matchesJson) as GlossaryMatch[];
  } catch {
    return [];
  }
}

export function parseAppError(err: unknown): AppErrorPayload {
  if (err && typeof err === "object") {
    const rec = err as Record<string, unknown>;
    if (typeof rec.code === "string" && typeof rec.message === "string") {
      return { code: rec.code, message: rec.message };
    }
    if (typeof rec.message === "string") {
      return { code: "INTERNAL_ERROR", message: rec.message };
    }
  }
  if (typeof err === "string") {
    try {
      const parsed = JSON.parse(err) as Partial<AppErrorPayload>;
      if (parsed.code && parsed.message) {
        return { code: parsed.code, message: parsed.message };
      }
    } catch {
      // Plain text error
    }
    return { code: "INTERNAL_ERROR", message: err };
  }
  return {
    code: "INTERNAL_ERROR",
    message: "An unknown error occurred.",
  };
}

export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(2)} GB`;
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(1)} MB`;
  return `${Math.round(bytes / 1024)} KB`;
}

export function formatVramGb(mb: number): string {
  return `${(mb / 1024).toFixed(1)} GB`;
}

export function formatEta(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "--:--";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  if (mins >= 60) {
    const hrs = Math.floor(mins / 60);
    const remMins = mins % 60;
    return `${String(hrs).padStart(2, "0")}:${String(remMins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

export const ipc = {
  scanHardware: () => invoke<HardwareProfile>("scan_hardware"),
  getEngineTelemetry: () => invoke<EngineTelemetry>("get_engine_telemetry"),
  setDiagnosticsTabOpen: (open: boolean) =>
    invoke<void>("set_diagnostics_tab_open", { open }),
  checkDiskSpace: (dirPath: string) =>
    invoke<DiskSpaceInfo>("check_disk_space", { dirPath }),
  unloadSidecarModel: () => invoke<EngineTelemetry>("unload_sidecar_model"),

  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: (settings: AppSettings) =>
    invoke<AppSettings>("update_settings", { settings }),

  listModels: () => invoke<ModelsOverview>("list_models"),
  confirmAndStartModelDownload: (
    modelId: string,
    targetDir: string,
    onProgress: (evt: DownloadProgressEvent) => void
  ) => {
    const channel = new Channel<DownloadProgressEvent>(onProgress);
    return invoke<DownloadTask>("confirm_and_start_model_download", {
      modelId,
      targetDir,
      onProgress: channel,
    });
  },
  pauseModelDownload: (modelId: string) =>
    invoke<void>("pause_model_download", { modelId }),
  resumeModelDownload: (
    modelId: string,
    onProgress: (evt: DownloadProgressEvent) => void
  ) => {
    const channel = new Channel<DownloadProgressEvent>(onProgress);
    return invoke<DownloadTask>("resume_model_download", {
      modelId,
      onProgress: channel,
    });
  },
  cancelModelDownload: (modelId: string) =>
    invoke<void>("cancel_model_download", { modelId }),
  importLocalGguf: (filePath: string) =>
    invoke<InstalledModelWithHealth>("import_local_gguf", { filePath }),
  relinkByomModel: (modelId: string, newFilePath: string) =>
    invoke<InstalledModelWithHealth>("relink_byom_model", {
      modelId,
      newFilePath,
    }),
  activateModel: (modelId: string) =>
    invoke<InstalledModelWithHealth>("activate_model", { modelId }),
  deleteModel: (modelId: string) => invoke<void>("delete_model", { modelId }),

  translateText: (
    sourceText: string,
    sourceLang: string,
    targetLang: string,
    tone: Tone,
    onStream: (evt: TranslationStreamEvent) => void
  ) => {
    const channel = new Channel<TranslationStreamEvent>(onStream);
    return invoke<TranslationResult>("translate_text", {
      sourceText,
      sourceLang,
      targetLang,
      tone,
      onStream: channel,
    });
  },
  cancelTextTranslation: () => invoke<void>("cancel_text_translation"),
  getWordAlternatives: (
    word: string,
    sentenceContext: string,
    sourceText: string,
    targetLang: string
  ) =>
    invoke<WordAlternativesResponse>("get_word_alternatives", {
      word,
      sentenceContext,
      sourceText,
      targetLang,
    }),
  saveTextHistory: (payload: {
    sourceText: string;
    translatedText: string;
    sourceLang: string;
    targetLang: string;
    tone: Tone;
    modelId: string;
    modelDisplayName: string;
    ttftMs: number | null;
    tokPerSec: number | null;
  }) => invoke<TextHistoryItem>("save_text_history", payload),
  listTextHistory: (query?: string, limit?: number) =>
    invoke<TextHistoryItem[]>("list_text_history", { query, limit }),
  deleteTextHistory: (id: string) =>
    invoke<void>("delete_text_history", { id }),

  listGlossaries: () => invoke<Glossary[]>("list_glossaries"),
  createGlossary: (name: string) =>
    invoke<Glossary>("create_glossary", { name }),
  updateGlossary: (id: string, name: string) =>
    invoke<Glossary>("update_glossary", { id, name }),
  deleteGlossary: (id: string) => invoke<void>("delete_glossary", { id }),
  listGlossaryEntries: (glossaryId: string) =>
    invoke<GlossaryEntry[]>("list_glossary_entries", { glossaryId }),
  upsertGlossaryEntry: (payload: {
    id?: string;
    glossaryId: string;
    sourceTerm: string;
    targetTerm: string;
    caseSensitive: boolean;
  }) => invoke<GlossaryEntry>("upsert_glossary_entry", payload),
  deleteGlossaryEntry: (id: string, glossaryId: string) =>
    invoke<void>("delete_glossary_entry", { id, glossaryId }),
  exportGlossary: (
    glossaryId: string,
    destPath: string,
    format?: "json" | "csv"
  ) =>
    invoke<string>("export_glossary", { glossaryId, destPath, format }),
  importGlossary: (payload: {
    filePath: string;
    targetGlossaryId?: string | null;
    collectionName?: string | null;
  }) => invoke<GlossaryImportResult>("import_glossary", payload),

  parseAndCreateSubtitleJob: (payload: {
    filePath: string;
    sourceLang?: string;
    targetLang?: string;
    genre?: SubtitleGenre;
    glossaryId?: string | null;
    tagShieldEnabled?: boolean;
  }) => invoke<SubtitleJobSummary>("parse_and_create_subtitle_job", payload),
  listSubtitleJobs: () => invoke<SubtitleJobSummary[]>("list_subtitle_jobs"),
  getSubtitleJob: (jobId: string) =>
    invoke<SubtitleJobSummary>("get_subtitle_job", { jobId }),
  deleteSubtitleJob: (jobId: string) =>
    invoke<void>("delete_subtitle_job", { jobId }),
  updateSubtitleJobOptions: (payload: {
    jobId: string;
    sourceLang?: string;
    targetLang?: string;
    genre: SubtitleGenre;
    glossaryId: string | null;
    tagShieldEnabled: boolean;
  }) => invoke<SubtitleJobSummary>("update_subtitle_job_options", payload),
  getSubtitleCues: (payload: {
    jobId: string;
    offset: number;
    limit: number;
    searchQuery?: string;
    statusFilter?: SubtitleCueFilter;
  }) => invoke<SubtitleCueDto[]>("get_subtitle_cues", payload),
  startSubtitleJob: (jobId: string) =>
    invoke<SubtitleJobSummary>("start_subtitle_job", { jobId }),
  pauseSubtitleJob: (jobId: string) =>
    invoke<SubtitleJobSummary>("pause_subtitle_job", { jobId }),
  resumeSubtitleJob: (jobId: string) =>
    invoke<SubtitleJobSummary>("resume_subtitle_job", { jobId }),
  cancelSubtitleJob: (jobId: string) =>
    invoke<SubtitleJobSummary>("cancel_subtitle_job", { jobId }),
  updateCueTranslation: (payload: {
    jobId: string;
    cueIndex: number;
    translatedText: string;
  }) => invoke<SubtitleCueDto>("update_cue_translation", payload),
  retranslateSingleCue: (jobId: string, cueIndex: number) =>
    invoke<SubtitleCueDto>("retranslate_single_cue", { jobId, cueIndex }),
  exportSubtitleFile: (jobId: string, destPath: string) =>
    invoke<string>("export_subtitle_file", { jobId, destPath }),

  onVramTick: (cb: (payload: VramTickPayload) => void): Promise<UnlistenFn> =>
    listen<VramTickPayload>("engine:vram_tick", (event) => cb(event.payload)),
  onEngineTelemetry: (cb: (payload: EngineTelemetry) => void): Promise<UnlistenFn> =>
    listen<EngineTelemetry>("engine:telemetry", (event) => cb(event.payload)),
  onEngineError: (cb: (payload: AppErrorPayload) => void): Promise<UnlistenFn> =>
    listen<AppErrorPayload>("engine:error", (event) => cb(event.payload)),
  onSubtitleBatchStarted: (
    cb: (payload: SubtitleBatchStartedEvent) => void
  ): Promise<UnlistenFn> =>
    listen<SubtitleBatchStartedEvent>("subtitle:batch_started", (event) =>
      cb(event.payload)
    ),
  onSubtitleBatchCompleted: (
    cb: (payload: SubtitleBatchCompletedEvent) => void
  ): Promise<UnlistenFn> =>
    listen<SubtitleBatchCompletedEvent>("subtitle:batch_completed", (event) =>
      cb(event.payload)
    ),
  onSubtitleJobError: (
    cb: (payload: SubtitleJobErrorEvent) => void
  ): Promise<UnlistenFn> =>
    listen<SubtitleJobErrorEvent>("subtitle:job_error", (event) =>
      cb(event.payload)
    ),
};
