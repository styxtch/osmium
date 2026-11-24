export interface LanguageOption {
  code: string;
  name: string;
  nativeName: string;
  bcp47: string;
  shortCode: string;
}

export const SUPPORTED_LANGUAGES: readonly LanguageOption[] = [
  {
    code: "auto",
    name: "Auto-detect",
    nativeName: "Auto-detect",
    bcp47: "auto",
    shortCode: "AUTO",
  },
  {
    code: "en",
    name: "English",
    nativeName: "English",
    bcp47: "en-US",
    shortCode: "EN",
  },
  {
    code: "id",
    name: "Indonesian",
    nativeName: "Bahasa Indonesia",
    bcp47: "id-ID",
    shortCode: "ID",
  },
  {
    code: "ja",
    name: "Japanese",
    nativeName: "日本語",
    bcp47: "ja-JP",
    shortCode: "JA",
  },
  {
    code: "ko",
    name: "Korean",
    nativeName: "한국어",
    bcp47: "ko-KR",
    shortCode: "KO",
  },
  {
    code: "zh",
    name: "Chinese",
    nativeName: "中文",
    bcp47: "zh-CN",
    shortCode: "ZH",
  },
  {
    code: "ms",
    name: "Malay",
    nativeName: "Bahasa Melayu",
    bcp47: "ms-MY",
    shortCode: "MS",
  },
  {
    code: "es",
    name: "Spanish",
    nativeName: "Español",
    bcp47: "es-ES",
    shortCode: "ES",
  },
  {
    code: "fr",
    name: "French",
    nativeName: "Français",
    bcp47: "fr-FR",
    shortCode: "FR",
  },
  {
    code: "de",
    name: "German",
    nativeName: "Deutsch",
    bcp47: "de-DE",
    shortCode: "DE",
  },
  {
    code: "ar",
    name: "Arabic",
    nativeName: "العربية",
    bcp47: "ar-SA",
    shortCode: "AR",
  },
  {
    code: "ru",
    name: "Russian",
    nativeName: "Русский",
    bcp47: "ru-RU",
    shortCode: "RU",
  },
  {
    code: "th",
    name: "Thai",
    nativeName: "ไทย",
    bcp47: "th-TH",
    shortCode: "TH",
  },
  {
    code: "vi",
    name: "Vietnamese",
    nativeName: "Tiếng Việt",
    bcp47: "vi-VN",
    shortCode: "VI",
  },
] as const;

export const SOURCE_LANGUAGES: readonly LanguageOption[] = SUPPORTED_LANGUAGES;

export const TARGET_LANGUAGES: readonly LanguageOption[] =
  SUPPORTED_LANGUAGES.filter((lang) => lang.code !== "auto");

const DEFAULT_LANGUAGE = SUPPORTED_LANGUAGES[1];

export function getLanguageInfo(code?: string | null): LanguageOption {
  const normalized = (code ?? "").trim().toLowerCase();
  if (!normalized) return DEFAULT_LANGUAGE;
  return (
    SUPPORTED_LANGUAGES.find(
      (lang) =>
        lang.code === normalized || lang.name.toLowerCase() === normalized
    ) ?? DEFAULT_LANGUAGE
  );
}

export function getLanguageLabel(code?: string | null): string {
  return getLanguageInfo(code).name;
}

export function getLanguageShortCode(code?: string | null): string {
  return getLanguageInfo(code).shortCode;
}

export function formatLanguagePair(
  sourceLang?: string | null,
  targetLang?: string | null
): string {
  const src = getLanguageShortCode(sourceLang || "en");
  const dst = getLanguageShortCode(targetLang || "id");
  return `${src} → ${dst}`;
}

export interface DetectedLanguage {
  code: string;
  strong: boolean;
}

const STOPWORDS: Record<"en" | "id" | "ms" | "es" | "fr" | "de", Set<string>> = {
  en: new Set([
    "the",
    "is",
    "are",
    "was",
    "were",
    "have",
    "has",
    "had",
    "do",
    "does",
    "did",
    "will",
    "would",
    "can",
    "could",
    "should",
    "what",
    "where",
    "when",
    "why",
    "who",
    "how",
    "this",
    "that",
    "these",
    "those",
    "with",
    "from",
    "for",
    "and",
    "but",
    "or",
    "not",
    "you",
    "your",
    "they",
    "their",
    "he",
    "she",
    "we",
    "it",
    "i",
    "i'm",
    "don't",
    "doesn't",
    "didn't",
    "it's",
    "that's",
    "there",
    "here",
    "about",
    "just",
    "need",
    "never",
    "really",
    "hello",
    "thanks",
  ]),
  id: new Set([
    "yang",
    "dan",
    "di",
    "ke",
    "dari",
    "ini",
    "itu",
    "untuk",
    "dengan",
    "tidak",
    "tak",
    "enggak",
    "nggak",
    "gak",
    "bisa",
    "dapat",
    "akan",
    "sudah",
    "udah",
    "belum",
    "sedang",
    "lagi",
    "aku",
    "kamu",
    "kita",
    "kami",
    "mereka",
    "dia",
    "atau",
    "tapi",
    "tetapi",
    "karena",
    "kalau",
    "jika",
    "juga",
    "hanya",
    "saja",
    "aja",
    "harus",
    "perlu",
    "kenapa",
    "mengapa",
    "bagaimana",
    "apa",
    "siapa",
    "semua",
    "sendirian",
    "banget",
    "bukan",
    "jangan",
    "tolong",
    "terima",
    "kasih",
  ]),
  ms: new Set([
    "awak",
    "kerana",
    "ialah",
    "adalah",
    "sahaja",
    "macam",
    "serta",
    "tersebut",
    "hendak",
    "tiada",
    "kalian",
    "beliau",
    "bahawa",
    "encik",
    "puan",
    "bagaimanakah",
    "bilakah",
    "mahukah",
    "selamat",
    "petang",
  ]),
  es: new Set([
    "el",
    "los",
    "las",
    "una",
    "unos",
    "unas",
    "del",
    "y",
    "que",
    "es",
    "son",
    "por",
    "para",
    "con",
    "pero",
    "como",
    "cómo",
    "qué",
    "dónde",
    "cuando",
    "cuándo",
    "yo",
    "tú",
    "él",
    "ella",
    "nosotros",
    "ellos",
    "esta",
    "este",
    "esto",
    "hola",
    "gracias",
    "muy",
    "tengo",
    "quiero",
    "estás",
    "hoy",
  ]),
  fr: new Set([
    "le",
    "les",
    "des",
    "du",
    "et",
    "est",
    "sont",
    "dans",
    "pour",
    "avec",
    "pas",
    "ne",
    "qui",
    "quoi",
    "où",
    "quand",
    "mais",
    "je",
    "tu",
    "il",
    "elle",
    "nous",
    "vous",
    "ils",
    "elles",
    "ce",
    "cette",
    "ces",
    "bonjour",
    "merci",
    "très",
    "suis",
    "comment",
    "allez",
    "c'est",
    "j'ai",
    "n'est",
    "aujourd'hui",
  ]),
  de: new Set([
    "der",
    "die",
    "das",
    "ein",
    "eine",
    "einen",
    "einem",
    "und",
    "ist",
    "sind",
    "war",
    "nicht",
    "ich",
    "du",
    "er",
    "sie",
    "wir",
    "ihr",
    "mit",
    "für",
    "auf",
    "aus",
    "von",
    "zu",
    "bei",
    "nach",
    "über",
    "wie",
    "was",
    "wo",
    "wann",
    "aber",
    "auch",
    "hallo",
    "danke",
    "sehr",
    "kann",
    "weiß",
  ]),
};

export function detectLanguageDetailed(text: string): DetectedLanguage | null {
  const trimmed = text.trim();
  if (!trimmed) return null;

  // Script-based detection (always high-confidence)
  if (/[\u3040-\u309F\u30A0-\u30FF]/.test(trimmed)) {
    return { code: "ja", strong: true };
  }
  if (/[\uAC00-\uD7AF\u1100-\u11FF\u3130-\u318F]/.test(trimmed)) {
    return { code: "ko", strong: true };
  }
  if (/[\u4E00-\u9FFF]/.test(trimmed)) {
    return { code: "zh", strong: true };
  }
  if (/[\u0E00-\u0E7F]/.test(trimmed)) {
    return { code: "th", strong: true };
  }
  if (/[\u0600-\u06FF]/.test(trimmed)) {
    return { code: "ar", strong: true };
  }
  if (/[\u0400-\u04FF]/.test(trimmed)) {
    return { code: "ru", strong: true };
  }

  // Latin-script detection
  const words =
    trimmed.toLowerCase().match(/[\p{L}]+(?:'[\p{L}]+)?/gu) ?? [];

  const viMatches = trimmed.match(
    /[ăđơưạảấầẩẫậắằẳẵặẹẻẽếềểễệỉịọỏốồổỗộớờởỡợụủứừửữựỳỵỷỹ]/gi
  );
  if (viMatches && viMatches.length > 0) {
    return {
      code: "vi",
      strong: viMatches.length >= 2 || words.length >= 2,
    };
  }

  const scores: Record<"en" | "id" | "ms" | "es" | "fr" | "de", number> = {
    en: 0,
    id: 0,
    ms: 0,
    es: 0,
    fr: 0,
    de: 0,
  };

  for (const word of words) {
    for (const lang of Object.keys(STOPWORDS) as Array<keyof typeof STOPWORDS>) {
      if (STOPWORDS[lang].has(word)) {
        scores[lang] += 1;
      }
    }
  }

  const hasSpanishPunct = /[¿¡]/.test(trimmed);
  const hasGermanSharpS = /ß/.test(trimmed);
  if (hasSpanishPunct) scores.es += 2;
  if (/ñ/i.test(trimmed)) scores.es += 1;
  if (hasGermanSharpS) scores.de += 2;
  if (/[äöü]/i.test(trimmed)) scores.de += 1;
  if (/[œçàâêîôû]/i.test(trimmed)) scores.fr += 1;

  const ranked = (
    Object.entries(scores) as Array<[keyof typeof scores, number]>
  ).sort((a, b) => b[1] - a[1]);

  const [bestLang, bestScore] = ranked[0];
  const secondScore = ranked[1]?.[1] ?? 0;

  if (bestScore < 1 || bestScore === secondScore) {
    return null;
  }

  const hasDistinctivePunctuation = hasSpanishPunct || hasGermanSharpS;
  const strong =
    bestScore >= 2 ||
    (bestScore >= 1 && bestScore - secondScore >= 1 && words.length >= 3) ||
    hasDistinctivePunctuation;

  return { code: bestLang, strong };
}

export function detectLanguage(text: string): string | null {
  return detectLanguageDetailed(text)?.code ?? null;
}

