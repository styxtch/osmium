import type { TtsProvider, TtsProviderId, TtsVoiceInfo } from "./types";
import { windowsNativeProvider } from "./windowsNativeProvider";

const SAMPLE_PHRASES: Record<string, string> = {
  en: "Hello, Osmium local translation is ready.",
  id: "Halo, terjemahan lokal Osmium siap digunakan.",
  ja: "こんにちは、Osmiumのローカル翻訳の準備ができました。",
  ko: "안녕하세요, Osmium 로컬 번역이 준비되었습니다.",
  zh: "你好，Osmium 本地翻译已准备就绪。",
  ms: "Helo, terjemahan tempatan Osmium sedia digunakan.",
  es: "Hola, la traducción local de Osmium está lista.",
  fr: "Bonjour, la traduction locale Osmium est prête.",
  de: "Hallo, die lokale Osmium-Übersetzung ist bereit.",
  ar: "مرحبا، الترجمة المحلية من Osmium جاهزة.",
  ru: "Здравствуйте, локальный перевод Osmium готов.",
  th: "สวัสดี ระบบแปลภาษาออฟไลน์ Osmium พร้อมใช้งานแล้ว",
  vi: "Xin chào, trình dịch cục bộ Osmium đã sẵn sàng.",
};

const providers = new Map<TtsProviderId, TtsProvider>([
  ["native", windowsNativeProvider],
]);

export function registerProvider(provider: TtsProvider): void {
  providers.set(provider.id, provider);
}

export function getProvider(id: TtsProviderId = "native"): TtsProvider {
  return providers.get(id) ?? windowsNativeProvider;
}

export function listVoices(
  providerId: TtsProviderId = "native",
  langCode?: string
): TtsVoiceInfo[] {
  return getProvider(providerId).listVoices(langCode);
}

export function stopAll(): void {
  for (const provider of providers.values()) {
    provider.stop();
  }
}

export function parseVoicePrefs(json?: string | null): Record<string, string> {
  if (!json) return {};
  try {
    const parsed = JSON.parse(json) as unknown;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return {};
    }
    const clean: Record<string, string> = {};
    for (const [k, v] of Object.entries(parsed)) {
      if (typeof v === "string" && v.trim()) {
        clean[k] = v;
      }
    }
    return clean;
  } catch {
    return {};
  }
}

export function serializeVoicePrefs(prefs: Record<string, string>): string {
  return JSON.stringify(prefs);
}

export function getSamplePhrase(langCode: string): string {
  return SAMPLE_PHRASES[langCode] ?? SAMPLE_PHRASES.en;
}
