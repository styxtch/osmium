export type TtsProviderId = "native" | "neural";

export interface TtsVoiceInfo {
  id: string;
  name: string;
  lang: string;
  localService: boolean;
  providerId: TtsProviderId;
}

export interface TtsSpeakOptions {
  text: string;
  langCode: string;
  rate?: number;
  preferredVoiceId?: string;
  onEnd?: () => void;
  onError?: (message: string) => void;
}

export interface TtsSpeakResult {
  ok: boolean;
  warning?: string;
  usedVoiceName?: string;
}

export interface TtsProvider {
  readonly id: TtsProviderId;
  readonly displayName: string;
  isAvailable: () => boolean;
  listVoices: (langCode?: string) => TtsVoiceInfo[];
  speak: (options: TtsSpeakOptions) => TtsSpeakResult;
  stop: () => void;
}
