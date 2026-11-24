use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct LanguageInfo {
    pub code: &'static str,
    pub name: &'static str,
    pub native_name: &'static str,
    pub bcp47: &'static str,
}

pub const SUPPORTED_LANGUAGES: &[LanguageInfo] = &[
    LanguageInfo {
        code: "auto",
        name: "Auto-detect",
        native_name: "Auto-detect",
        bcp47: "auto",
    },
    LanguageInfo {
        code: "en",
        name: "English",
        native_name: "English",
        bcp47: "en-US",
    },
    LanguageInfo {
        code: "id",
        name: "Indonesian",
        native_name: "Bahasa Indonesia",
        bcp47: "id-ID",
    },
    LanguageInfo {
        code: "ja",
        name: "Japanese",
        native_name: "日本語",
        bcp47: "ja-JP",
    },
    LanguageInfo {
        code: "ko",
        name: "Korean",
        native_name: "한국어",
        bcp47: "ko-KR",
    },
    LanguageInfo {
        code: "zh",
        name: "Chinese",
        native_name: "中文",
        bcp47: "zh-CN",
    },
    LanguageInfo {
        code: "ms",
        name: "Malay",
        native_name: "Bahasa Melayu",
        bcp47: "ms-MY",
    },
    LanguageInfo {
        code: "es",
        name: "Spanish",
        native_name: "Español",
        bcp47: "es-ES",
    },
    LanguageInfo {
        code: "fr",
        name: "French",
        native_name: "Français",
        bcp47: "fr-FR",
    },
    LanguageInfo {
        code: "de",
        name: "German",
        native_name: "Deutsch",
        bcp47: "de-DE",
    },
    LanguageInfo {
        code: "ar",
        name: "Arabic",
        native_name: "العربية",
        bcp47: "ar-SA",
    },
    LanguageInfo {
        code: "ru",
        name: "Russian",
        native_name: "Русский",
        bcp47: "ru-RU",
    },
    LanguageInfo {
        code: "th",
        name: "Thai",
        native_name: "ไทย",
        bcp47: "th-TH",
    },
    LanguageInfo {
        code: "vi",
        name: "Vietnamese",
        native_name: "Tiếng Việt",
        bcp47: "vi-VN",
    },
];

pub fn normalize_source_lang(raw: Option<&str>) -> &'static str {
    let candidate = raw.unwrap_or("en").trim().to_ascii_lowercase();
    SUPPORTED_LANGUAGES
        .iter()
        .find(|l| l.code == candidate || l.name.eq_ignore_ascii_case(&candidate))
        .map(|l| l.code)
        .unwrap_or("en")
}

pub fn normalize_target_lang(raw: Option<&str>) -> &'static str {
    let candidate = raw.unwrap_or("id").trim().to_ascii_lowercase();
    SUPPORTED_LANGUAGES
        .iter()
        .filter(|l| l.code != "auto")
        .find(|l| l.code == candidate || l.name.eq_ignore_ascii_case(&candidate))
        .map(|l| l.code)
        .unwrap_or("id")
}

pub fn language_info(code: &str) -> LanguageInfo {
    let lower = code.trim().to_ascii_lowercase();
    SUPPORTED_LANGUAGES
        .iter()
        .copied()
        .find(|l| l.code == lower)
        .unwrap_or(SUPPORTED_LANGUAGES[1])
}

pub fn language_name(code: &str) -> &'static str {
    language_info(code).name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_source_and_target_languages_safely() {
        assert_eq!(normalize_source_lang(None), "en");
        assert_eq!(normalize_source_lang(Some("auto")), "auto");
        assert_eq!(normalize_source_lang(Some("JA")), "ja");
        assert_eq!(normalize_source_lang(Some("unknown")), "en");

        assert_eq!(normalize_target_lang(None), "id");
        assert_eq!(normalize_target_lang(Some("auto")), "id");
        assert_eq!(normalize_target_lang(Some("English")), "en");
        assert_eq!(normalize_target_lang(Some("ko")), "ko");
    }
}
