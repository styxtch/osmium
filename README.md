# Osmium — Local AI Translator (Offline)

**Osmium** is a 100% local, zero-egress desktop AI translator for Windows 10/11 (x64), built with **Tauri v2**, **Rust**, **React 19**, **TypeScript**, **Tailwind CSS v4**, and **SQLite (WAL)** powered by a bundled **`llama-server` (Vulkan / CPU)** sidecar.

Translate everyday text and multi-thousand-line `.srt`, `.vtt`, and `.ass` subtitle files completely offline on your GPU—with no cloud APIs, no telemetry, and no user content written to disk logs.

---

## Features (v1.1.0)

### 1. Multi-Language Text Translator (`Auto-detect` + 13 Languages + `⇄` Swap)
- **Bidirectional & Multi-Language (`EN`, `ID`, `JA`, `KO`, `ZH`, `MS`, `ES`, `FR`, `DE`, `AR`, `RU`, `TH`, `VI`):** Real-time offline source language detection updates dropdown labels and pane headers automatically as you type or paste, with 1-click bidirectional `⇄` swap.
- **Live SSE Streaming:** Real-time token streaming from `llama-server` with unified `tok/s` + `TTFT` latency pill telemetry and no redundant re-translation on tab switches.
- **Adaptive `Natural` Tone + Explicit Registers:** `Natural` auto-adapts across novel prose, quoted dialogue, and formal writing, while `Formal` and `Casual` offer explicit register overrides with varying temperatures.
- **Auto-Translate & Manual Control:** Optional 600 ms debounce auto-translate or manual trigger (`Ctrl+Enter`).
- **Auto-Chunking & Safety Limit:** Seamless paragraph/sentence chunking above 5,000 characters (2,500 chars on 2048 ctx) up to a hard 25,000-character limit.
- **Word Alternatives & Undo Stack:** Click any translated word after completion to inspect 3–4 contextual synonyms and nuances in the active target language, replace inline preserving casing, and undo via `Ctrl+Z`.
- **Pluggable Offline Text-to-Speech (TTS):** Listen/Stop controls powered by zero-overhead Windows Native OS voices by default, with per-language voice preference, adjustable speed (`0.75x–1.50x`), and a pluggable `TtsProvider` slot ready for future neural voices.
- **Quick History:** Anti-spam local SQLite text history (`Ctrl+Shift+C` copy, language pair badges, quick reload).

### 2. Subtitle Translator (`.srt`, `.vtt`, `.ass` / `.ssa`)
- **Multi-Language Source & Target:** Translate subtitle files between any supported language pair (`Auto-detect` supported) with language-tagged export filenames (e.g., `episode01.id.ass`, `episode01.en.srt`).
- **Deterministic Round-Trip Engine:** Preserves `.ass` `[Script Info]`, `[V4+ Styles]`, `Format:` column order, dialogue commas, `Comment:` rows, vector drawing lines (`\p1`..`\p9`), and `[Fonts]`/`[Graphics]` footers; preserves `.vtt` `WEBVTT`/`STYLE`/`REGION`/`NOTE` blocks and cue settings; auto-detects UTF-8, BOM, UTF-16 LE/BE, and legacy encodings via `chardetng` + `encoding_rs`.
- **Rust Tag Shield:** Deterministically shields outer ASS/HTML tags and replaces inline markup/breaks with verified `<tN>...</tN>` and `<tN/>` placeholders before LLM inference.
- **Glossary Enforcement + JSON/CSV Import & Export:** Optional Unicode word-boundary glossary injection across batches (supports case-sensitive toggles), plus `.json` and UTF-8 `.csv` glossary import/export.
- **Bounded Self-Healing Pipeline:** Adaptive batch sizes (`18` / `12` / `6` by `--ctx-size`), 1–3 previous + 1–2 lookahead context cues, 1x strict retry, and recursive binary split fallback down to single cues—marking unrecoverable cues as `warning` while keeping original text intact.
- **Resumable SQLite WAL Checkpoints:** Commits every completed batch atomically with `Pause`, `Resume`, `Cancel`, and crash recovery.
- **60 FPS Virtualized Cue Workspace:** `@tanstack/react-virtual` table with `Ctrl+F` search/filtering, inline tag-safe editing, single-row re-translation, row TTS playback, and a **100%-completion lock** on Windows *Save As* export (`Ctrl+S`).

### 3. Model Manager, Hardware Telemetry & Memory Management
- **Windows DXGI + Vulkan LUID Mapping + NVML:** Accurately identifies discrete GPUs on hybrid laptops and displays live synchronized GPU VRAM, temperature, clock speed, system RAM usage, and recommended tier in both the sidebar and Hardware Diagnostics.
- **Consent-First Official Downloads:** Download verified `Qwen 3 8B` or `Qwen 3 4B` (`Q4_K_M`) GGUF weights from HuggingFace with strict HTTPS domain allowlisting, resumable HTTP Range support, disk-space checks, and SHA-256 verification—zero network activity unless explicitly approved.
- **BYOM (Bring Your Own Model):** Reference local `.gguf` files directly from disk with GGUF header/layer inspection and path relinking if moved.
- **Lazy Load, Auto-Idle Unload & Manual Unload:** `llama-server.exe` stays in `Standby` until the first translation request, runs under a Windows `Win32 Job Object` (`KILL_ON_JOB_CLOSE`), frees system RAM after full GPU offload via `--load-mode none`, and automatically or manually unloads after idle inactivity (default `10` min) to reclaim VRAM.

---

## Privacy & Security Architecture

- **Zero Runtime Telemetry:** No analytics, crash reporters, or background network calls.
- **Zero User Text in Logs:** `tracing` logs only operational metadata (port, PID, model ID)—never source text, translations, dialogue cues, or glossary terms.
- **Strict Boundaries:** Locked WebView CSP (`default-src 'self'`), minimal Tauri capabilities, `127.0.0.1` loopback binding with `no_proxy()`, and path-traversal protection against Windows system directories.

---

## System Requirements

- **OS:** Windows 10 / Windows 11 (64-bit / x64)
- **GPU (Recommended):** Vulkan-capable GPU with `≥ 6 GB` VRAM (`Qwen 3 4B`) or `≥ 8 GB` VRAM (`Qwen 3 8B`); CPU fallback supported
- **RAM:** `16 GB` recommended (`8 GB` minimum for 4B)
- **Disk Space:** ~`3–6 GB` free depending on chosen GGUF model

---

## Installation & Uninstallation (Windows)

Download either installer from **GitHub Releases**:
- `Osmium_<version>_x64-setup.exe` (NSIS Installer)
- `Osmium_<version>_x64_en-US.msi` (Windows Installer MSI)

To **uninstall**, open **Windows Settings → Apps → Installed apps**, search for **Osmium**, and select **Uninstall**.
User databases (`osmium.db`), logs, and default downloaded models live in `%LOCALAPPDATA%\com.osmium.translator` and are preserved across updates (delete that folder manually if you want a full cleanup).

---

## Local Development Setup

### Prerequisites
- [Node.js](https://nodejs.org/) `22+`
- [Rust](https://rustup.rs/) (`stable-x86_64-pc-windows-msvc`) + Visual Studio Build Tools (C++)
- Bundled `llama.cpp` Vulkan binaries (`b12651` — excluded from Git via `.gitignore`)

### 1. Place `llama-server.exe` & Runtime DLLs
Download [`llama-b12651-bin-win-vulkan-x64.zip`](https://github.com/ggml-org/llama.cpp/releases/tag/b12651) from `ggml-org/llama.cpp` and extract `llama-server.exe` along with all `*.dll` files into:

```text
src-tauri/resources/bin/
```

### 2. Install Dependencies & Run Dev Mode
```bash
npm install
npm run tauri dev
```
> **Windows PowerShell note:** If PowerShell blocks `npm.ps1` via ExecutionPolicy, run `npm.cmd run tauri dev` (or use Git Bash / CMD).

### 3. Run Tests & Production Build
```bash
# Frontend tests & bundle build
npm run test
npm run build

# Rust backend checks & tests
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib

# Build Windows NSIS (.exe) & MSI (.msi) installers
npm run tauri build
```

Generated installers will be located at:
- `src-tauri/target/release/bundle/nsis/Osmium_1.1.0_x64-setup.exe`
- `src-tauri/target/release/bundle/msi/Osmium_1.1.0_x64_en-US.msi`

---

## Project Structure

```text
osmium/
├── .github/workflows/release.yml   # Windows CI test + NSIS/MSI release pipeline
├── src/                            # React 19 + TypeScript + Tailwind CSS v4 frontend
│   ├── components/layout/          # AppShell, Sidebar, ActiveGpuCard
│   ├── features/
│   │   ├── text-translator/        # Dual-pane translator, tokenizer, word alternatives
│   │   ├── subtitle-translator/    # Dropzone, Spec Card, Search Bar, Virtual Table
│   │   ├── model-manager/          # DXGI+Vulkan+NVML diagnostics, catalog, BYOM
│   │   ├── glossary-history/       # Glossary & text history CRUD
│   │   └── settings/               # Display mode, folders, sidecar tuning
│   ├── lib/ipc.ts                  # Typed Tauri v2 invoke/Channel/event bindings
│   └── stores/                     # Zustand stores
└── src-tauri/                      # Tauri v2 Rust backend
    ├── resources/bin/              # Sidecar placement directory for llama-server.exe + DLLs
    └── src/
        ├── commands/               # Tauri IPC handlers
        ├── core/
        │   ├── engine/             # SidecarManager, Win32 JobObject, InferenceCoordinator
        │   ├── hardware/           # DXGI, Vulkan LUID mapper, NVML telemetry, disk checks
        │   ├── models/             # Catalog, HTTP Range downloader, GGUF validator
        │   ├── pipeline/           # Bounded Self-Healing orchestrator & batch translator
        │   ├── subtitle/           # Encoding, SRT/VTT/ASS round-trip, Tag Shield, exporter
        │   └── translator/         # Prompts, chunker, SSE streamer, word alternatives
        └── db/                     # SQLite WAL migrations & repositories
```

---

## Roadmap

- **v1.0.0:** Offline `EN → ID` Text Translator, Word Alternatives, `.srt`/`.vtt`/`.ass` Subtitle Translator with Tag Shield & Bounded Self-Healing, Windows NSIS & MSI installers.
- **v1.1.0 (Current):** Multi-language & bidirectional translation (`Auto-detect` + 13 languages), adaptive `Natural` tone, pluggable Windows Native offline TTS, auto-idle & manual VRAM unload, live sidebar temp & RAM telemetry, and `.json`/`.csv` Glossary Import/Export.
- **v1.2.0 (Planned):** Optional downloadable Local Neural AI TTS voices (Piper / Kokoro ONNX) alongside Windows Native TTS.
