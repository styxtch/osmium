import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSettingsStore } from "../../../stores/useSettingsStore";
import { useTtsStore } from "../../../stores/useTtsStore";
import {
  getProvider,
  parseVoicePrefs,
  serializeVoicePrefs,
} from "../ttsManager";
import {
  stripSpeechTags,
  WindowsNativeTtsProvider,
} from "../windowsNativeProvider";

interface MockVoice {
  voiceURI: string;
  name: string;
  lang: string;
  localService: boolean;
  default?: boolean;
}

class MockUtterance {
  text: string;
  lang = "";
  rate = 1;
  voice: SpeechSynthesisVoice | null = null;
  onend: (() => void) | null = null;
  onerror: ((evt: { error?: string }) => void) | null = null;

  constructor(text: string) {
    this.text = text;
  }
}

describe("Offline TTS Manager & WindowsNativeTtsProvider", () => {
  const originalSynth = window.speechSynthesis;
  const originalUtterance = globalThis.SpeechSynthesisUtterance;
  let spokenUtterances: MockUtterance[] = [];
  let cancelMock = vi.fn();
  let speakMock = vi.fn();
  let mockVoices: MockVoice[] = [];

  beforeEach(() => {
    spokenUtterances = [];
    cancelMock = vi.fn();
    speakMock = vi.fn((utt: MockUtterance) => {
      spokenUtterances.push(utt);
    });
    mockVoices = [
      {
        voiceURI: "ms-en-us",
        name: "Microsoft David - English (United States)",
        lang: "en-US",
        localService: true,
      },
      {
        voiceURI: "ms-en-gb",
        name: "Microsoft Hazel - English (Great Britain)",
        lang: "en-GB",
        localService: true,
      },
      {
        voiceURI: "ms-id-cloud",
        name: "Microsoft Andika Cloud",
        lang: "id-ID",
        localService: false,
      },
      {
        voiceURI: "ms-id-local",
        name: "Microsoft Gadis Local",
        lang: "id-ID",
        localService: true,
      },
    ];

    Object.defineProperty(window, "speechSynthesis", {
      configurable: true,
      value: {
        getVoices: () => mockVoices,
        speak: speakMock,
        cancel: cancelMock,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      },
    });
    (globalThis as unknown as Record<string, unknown>).SpeechSynthesisUtterance =
      MockUtterance;

    useSettingsStore.setState({
      settings: {
        display_mode: "standard",
        default_model_dir: "C:\\Models",
        default_export_dir: "C:\\Exports",
        backend_override: "auto",
        ctx_size_override: "auto",
        gpu_layers_override: "auto",
        tts_provider: "native",
        tts_rate: 1.15,
        tts_voice_prefs_json: "{}",
        auto_idle_unload_minutes: 10,
      },
    });
    useTtsStore.setState({ speakingKey: null, voices: [], notice: null });
  });

  afterEach(() => {
    Object.defineProperty(window, "speechSynthesis", {
      configurable: true,
      value: originalSynth,
    });
    (globalThis as unknown as Record<string, unknown>).SpeechSynthesisUtterance =
      originalUtterance;
  });

  it("strips ASS override tags, HTML tags, and backslash line breaks before speaking", () => {
    const raw = "{\\an8\\pos(100,50)}<i>Hello\\Nworld!</i>\\hLine\\ntwo";
    expect(stripSpeechTags(raw)).toBe("Hello world! Line two");
  });

  it("matches preferred voice first, then local exact BCP-47, then primary prefix, and warns if missing", () => {
    const provider = new WindowsNativeTtsProvider();

    // 1. Exact preferred voice ID override
    const prefRes = provider.speak({
      text: "Good morning",
      langCode: "en",
      preferredVoiceId: "ms-en-gb",
    });
    expect(prefRes.ok).toBe(true);
    expect(prefRes.usedVoiceName).toBe(
      "Microsoft Hazel - English (Great Britain)"
    );
    expect((spokenUtterances[0].voice as unknown as MockVoice).voiceURI).toBe(
      "ms-en-gb"
    );

    // 2. Exact BCP-47 prefers localService
    const exactLocal = provider.speak({
      text: "Selamat pagi",
      langCode: "id",
    });
    expect(exactLocal.ok).toBe(true);
    expect(exactLocal.usedVoiceName).toBe("Microsoft Gadis Local");

    // 3. Primary language prefix fallback when exact region absent
    mockVoices = [
      {
        voiceURI: "ms-es-mx",
        name: "Microsoft Sabina - Spanish (Mexico)",
        lang: "es-MX",
        localService: true,
      },
    ];
    const prefixRes = provider.speak({
      text: "Hola mundo",
      langCode: "es",
    });
    expect(prefixRes.ok).toBe(true);
    expect(prefixRes.usedVoiceName).toBe(
      "Microsoft Sabina - Spanish (Mexico)"
    );

    // 4. Missing voice pack returns gentle non-fatal warning
    mockVoices = [
      {
        voiceURI: "ms-en-us",
        name: "Microsoft David",
        lang: "en-US",
        localService: true,
      },
    ];
    const missingRes = provider.speak({
      text: "おはようございます",
      langCode: "ja",
    });
    expect(missingRes.ok).toBe(true);
    expect(missingRes.warning).toMatch(/No Windows voice pack found/i);
    expect(spokenUtterances[3].lang).toBe("ja-JP");
  });

  it("manages speak, toggle-stop, and switch-key lifecycle in useTtsStore and falls back when neural provider is unimplemented", () => {
    expect(getProvider("neural").id).toBe("native");
    expect(parseVoicePrefs(serializeVoicePrefs({ id: "ms-id-local" }))).toEqual({
      id: "ms-id-local",
    });

    const store = useTtsStore.getState();
    store.speak("target", "{\\i1}Halo dunia{\\i0}", "id");
    expect(useTtsStore.getState().speakingKey).toBe("target");
    expect(speakMock).toHaveBeenCalledTimes(1);
    expect(spokenUtterances[0].text).toBe("Halo dunia");
    expect(spokenUtterances[0].rate).toBeCloseTo(1.15);

    // Clicking the same key stops playback
    useTtsStore.getState().speak("target", "Halo dunia", "id");
    expect(useTtsStore.getState().speakingKey).toBeNull();
    expect(cancelMock).toHaveBeenCalled();

    // Switching to another key replaces speakingKey and respects onend completion
    useTtsStore.getState().speak("cue-1", "Baris pertama", "id");
    expect(useTtsStore.getState().speakingKey).toBe("cue-1");
    spokenUtterances[1].onend?.();
    expect(useTtsStore.getState().speakingKey).toBeNull();
  });
});
