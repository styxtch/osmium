import { getLanguageInfo, getLanguageLabel } from "../languages";
import type {
  TtsProvider,
  TtsSpeakOptions,
  TtsSpeakResult,
  TtsVoiceInfo,
} from "./types";

export function stripSpeechTags(text: string): string {
  return text
    .replace(/\\N|\\n|\\h/g, " ")
    .replace(/\{[^}]*\}/g, "")
    .replace(/<[^>]+>/g, "")
    .replace(/[ \t]+/g, " ")
    .replace(/\s*\r?\n\s*/g, " ")
    .trim();
}

function normalizeLangTag(tag: string): string {
  return tag.trim().toLowerCase().replace(/_/g, "-");
}

function matchVoiceForLang(
  voices: SpeechSynthesisVoice[],
  bcp47: string,
  preferredVoiceId?: string
): SpeechSynthesisVoice | undefined {
  if (preferredVoiceId) {
    const exactPref = voices.find(
      (v) => v.voiceURI === preferredVoiceId || v.name === preferredVoiceId
    );
    if (exactPref) return exactPref;
  }

  const targetNorm = normalizeLangTag(bcp47);
  const targetPrimary = targetNorm.split("-")[0];

  const exactMatches = voices.filter(
    (v) => normalizeLangTag(v.lang) === targetNorm
  );
  if (exactMatches.length > 0) {
    return exactMatches.find((v) => v.localService) ?? exactMatches[0];
  }

  const prefixMatches = voices.filter((v) => {
    const vNorm = normalizeLangTag(v.lang);
    return vNorm === targetPrimary || vNorm.startsWith(`${targetPrimary}-`);
  });
  if (prefixMatches.length > 0) {
    return prefixMatches.find((v) => v.localService) ?? prefixMatches[0];
  }

  return undefined;
}

export class WindowsNativeTtsProvider implements TtsProvider {
  readonly id = "native" as const;
  readonly displayName = "Windows Native (Default)";

  isAvailable(): boolean {
    return (
      typeof window !== "undefined" &&
      "speechSynthesis" in window &&
      typeof window.speechSynthesis?.speak === "function" &&
      typeof SpeechSynthesisUtterance !== "undefined"
    );
  }

  listVoices(langCode?: string): TtsVoiceInfo[] {
    if (!this.isAvailable()) return [];

    const raw = window.speechSynthesis.getVoices?.() ?? [];
    const mapped: TtsVoiceInfo[] = raw.map((v) => ({
      id: v.voiceURI || v.name,
      name: v.name,
      lang: v.lang,
      localService: Boolean(v.localService),
      providerId: "native",
    }));

    if (!langCode || langCode === "auto") {
      return mapped;
    }

    const bcp47 = getLanguageInfo(langCode).bcp47;
    const primary = normalizeLangTag(bcp47).split("-")[0];

    return mapped.filter((v) => {
      const vNorm = normalizeLangTag(v.lang);
      return vNorm === primary || vNorm.startsWith(`${primary}-`);
    });
  }

  speak(options: TtsSpeakOptions): TtsSpeakResult {
    const cleaned = stripSpeechTags(options.text);
    if (!cleaned) {
      options.onEnd?.();
      return { ok: true };
    }

    if (!this.isAvailable()) {
      const msg =
        "Windows text-to-speech is not available in this environment.";
      options.onError?.(msg);
      return { ok: false, warning: msg };
    }

    const synth = window.speechSynthesis;
    synth.cancel();

    const effectiveCode =
      options.langCode === "auto" || !options.langCode ? "en" : options.langCode;
    const bcp47 = getLanguageInfo(effectiveCode).bcp47;
    const allVoices = synth.getVoices?.() ?? [];
    const matched = matchVoiceForLang(
      allVoices,
      bcp47,
      options.preferredVoiceId
    );

    const utterance = new SpeechSynthesisUtterance(cleaned);
    utterance.lang = matched?.lang || bcp47;
    utterance.rate = Math.min(2.0, Math.max(0.5, options.rate ?? 1.0));
    if (matched) {
      utterance.voice = matched;
    }

    utterance.onend = () => {
      options.onEnd?.();
    };
    utterance.onerror = (evt) => {
      const errCode = (evt as { error?: string } | undefined)?.error;
      if (errCode === "canceled" || errCode === "interrupted") {
        options.onEnd?.();
        return;
      }
      options.onError?.(
        `Speech synthesis failed${errCode ? ` (${errCode})` : ""}.`
      );
    };

    synth.speak(utterance);

    if (!matched) {
      return {
        ok: true,
        warning: `No Windows voice pack found for ${getLanguageLabel(effectiveCode)} (${bcp47}). Install the speech pack under Windows Settings > Time & Language > Speech if audio is silent.`,
      };
    }

    return {
      ok: true,
      usedVoiceName: matched.name,
    };
  }

  stop(): void {
    if (typeof window !== "undefined" && window.speechSynthesis?.cancel) {
      window.speechSynthesis.cancel();
    }
  }
}

export const windowsNativeProvider = new WindowsNativeTtsProvider();
