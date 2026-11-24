import { create } from "zustand";
import {
  getProvider,
  listVoices,
  parseVoicePrefs,
  stopAll,
} from "../lib/tts/ttsManager";
import type { TtsProviderId, TtsVoiceInfo } from "../lib/tts/types";
import { useSettingsStore } from "./useSettingsStore";

interface TtsSpeakOverrides {
  providerId?: TtsProviderId;
  rate?: number;
  preferredVoiceId?: string;
}

interface TtsState {
  speakingKey: string | null;
  voices: TtsVoiceInfo[];
  notice: string | null;
  refreshVoices: () => void;
  speak: (
    key: string,
    text: string,
    langCode: string,
    overrides?: TtsSpeakOverrides
  ) => void;
  stop: () => void;
  clearNotice: () => void;
}

let activeSpeakSeq = 0;

export const useTtsStore = create<TtsState>((set, get) => ({
  speakingKey: null,
  voices: [],
  notice: null,

  refreshVoices: () => {
    const providerRaw = useSettingsStore.getState().settings?.tts_provider;
    const providerId: TtsProviderId =
      providerRaw === "neural" ? "neural" : "native";
    const voices = listVoices(providerId);
    set({ voices });
  },

  speak: (key, text, langCode, overrides) => {
    if (get().speakingKey === key) {
      get().stop();
      return;
    }

    const settings = useSettingsStore.getState().settings;
    const providerRaw = overrides?.providerId ?? settings?.tts_provider;
    const providerId: TtsProviderId =
      providerRaw === "neural" ? "neural" : "native";
    const rate = overrides?.rate ?? settings?.tts_rate ?? 1.0;
    const voicePrefs = parseVoicePrefs(settings?.tts_voice_prefs_json);
    const preferredVoiceId =
      overrides?.preferredVoiceId !== undefined
        ? overrides.preferredVoiceId || undefined
        : voicePrefs[langCode];

    const provider = getProvider(providerId);
    const seq = ++activeSpeakSeq;

    set({ speakingKey: key, notice: null });

    const result = provider.speak({
      text,
      langCode,
      rate,
      preferredVoiceId,
      onEnd: () => {
        if (activeSpeakSeq === seq) {
          set({ speakingKey: null });
        }
      },
      onError: (message) => {
        if (activeSpeakSeq === seq) {
          set({ speakingKey: null, notice: message });
        }
      },
    });

    if (!result.ok) {
      set({
        speakingKey: null,
        notice: result.warning ?? "Text-to-speech playback could not start.",
      });
      return;
    }

    if (result.warning) {
      set({ notice: result.warning });
    }
  },

  stop: () => {
    activeSpeakSeq += 1;
    stopAll();
    set({ speakingKey: null });
  },

  clearNotice: () => set({ notice: null }),
}));
