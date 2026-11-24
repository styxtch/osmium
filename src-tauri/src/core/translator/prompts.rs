use crate::core::engine::llama_client::ChatMessage;
use crate::core::translator::languages::{language_info, language_name};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Natural,
    Formal,
    Casual,
}

impl Tone {
    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "formal" => Self::Formal,
            "casual" | "santai" => Self::Casual,
            _ => Self::Natural,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Natural => "natural",
            Self::Formal => "formal",
            Self::Casual => "casual",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Genre {
    General,
    Anime,
    Film,
    Documentary,
    Gaming,
}

impl Genre {
    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "anime" => Self::Anime,
            "film" => Self::Film,
            "documentary" | "dokumenter" => Self::Documentary,
            "gaming" => Self::Gaming,
            _ => Self::General,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Anime => "anime",
            Self::Film => "film",
            Self::Documentary => "documentary",
            Self::Gaming => "gaming",
        }
    }

    pub fn prompt_instruction(self) -> &'static str {
        match self {
            Self::General => {
                "Gunakan Bahasa Indonesia yang natural sesuai konteks sumber. \
                 Untuk percakapan sehari-hari gunakan pronomina akrab (aku/kamu), \
                 sedangkan saya/Anda hanya jika konteks aslinya jelas resmi atau formal."
            }
            Self::Anime => {
                "Gaya dialog anime/animasi: ekspresif, ringkas sesuai tempo suara, gunakan \
                 pronomina dinamis (aku/kamu/kau) yang wajar dalam dialog lisan, dan \
                 pertahankan honorifik Jepang (-san, -kun, -chan, -sama, senpai, sensei) jika ada pada teks sumber."
            }
            Self::Film => {
                "Gaya subtitle film & serial TV (standar bioskop/streaming): kalimat singkat, lisan alami, dan enak dibaca sekilas. \
                 WAJIB gunakan pronomina akrab (aku/kamu, namaku, kau) secara default untuk dialog personal/sehari-hari; \
                 gunakan saya/Anda HANYA jika situasinya jelas resmi, rapat kantor, atau menyapa otoritas asing. \
                 Hindari kata tulisan kaku seperti 'apakah', 'tersebut', atau 'Halo, nama saya...' pada perkenalan santai."
            }
            Self::Documentary => {
                "Gaya dokumenter realistis: informatif dengan Bahasa Indonesia baku EYD, gunakan pronomina resmi \
                 (saya, Anda, mereka, kita) dan istilah ilmiah/sejarah yang akurat. \
                 DILARANG menggunakan kata gaul atau singkatan informal (seperti 'gak', 'udah', 'banget', 'sih')."
            }
            Self::Gaming => {
                "Gaya gaming & lore: dialog karakter tetap hidup dengan pronomina natural (aku/kamu/kau), \
                 serta pertahankan istilah mekanik atau dunia game yang umum dikenal pemain \
                 (seperti HP, mana, boss, quest, guild, buff, skill, item, respawn)."
            }
        }
    }

    pub fn prompt_instruction_for_lang(self, target_lang: &str) -> String {
        if target_lang == "id" {
            return self.prompt_instruction().to_string();
        }

        if target_lang == "en" {
            return match self {
                Self::General => {
                    "Use natural, idiomatic English subtitles that preserve the speaker's original tone and formality."
                }
                Self::Anime => {
                    "Anime dialogue style: expressive, concise to match voice timing, natural spoken phrasing, and keep Japanese honorifics (-san, -kun, -chan, -sama, senpai, sensei) when present in source."
                }
                Self::Film => {
                    "Cinema & TV streaming subtitle style: punchy, natural spoken English using realistic contractions (I'm, don't, what's); keep formal phrasing only for clearly official scenes."
                }
                Self::Documentary => {
                    "Documentary style: informative, authoritative standard English with accurate scientific and historical terminology and zero slang."
                }
                Self::Gaming => {
                    "Gaming & lore style: lively dialogue keeping standard gaming/RPG terminology intact (HP, mana, boss, quest, guild, buff, skill, respawn)."
                }
            }
            .to_string();
        }

        let info = language_info(target_lang);
        let style_desc = match self {
            Self::General => "natural subtitle phrasing matching source register",
            Self::Anime => "expressive anime dialogue concise to speech tempo, keeping source honorifics where appropriate",
            Self::Film => "cinema/streaming subtitle dialogue: concise, spoken, and natural to read quickly",
            Self::Documentary => "informative standard documentary style with accurate technical/historical terms and zero slang",
            Self::Gaming => "lively gaming & lore dialogue preserving common game mechanic terms",
        };
        format!(
            "Target language: {} ({}). Subtitle style: {}.",
            info.name, info.native_name, style_desc
        )
    }
}

pub fn build_text_translation_messages(
    source_chunk: &str,
    source_lang: &str,
    target_lang: &str,
    tone: Tone,
) -> Vec<ChatMessage> {
    let source_label = if source_lang == "auto" {
        "automatically detected source language"
    } else {
        language_name(source_lang)
    };

    let system = if target_lang == "id" {
        let tone_guide = match tone {
            Tone::Natural => {
                "Gaya NATURAL (Adaptif seperti DeepL): Sesuaikan register Bahasa Indonesia secara otomatis dengan konteks tiap bagian teks sumber — \
                 narasi novel/prosa tetap mengalir, emosional, dan luwes; dialog di dalam tanda kutip mengikuti kedekatan pembicara \
                 (akrab aku/kamu bila personal, saya/Anda bila situasinya resmi); serta dokumen formal tetap rapi tanpa terdengar seperti robot."
            }
            Tone::Formal => {
                "Gaya FORMAL (Baku EYD/PUEBI): WAJIB gunakan kata ganti resmi (saya, Anda, beliau) dan kosakata baku lengkap \
                 (tidak, sudah, tetapi, bagaimana, mengapa). DILARANG KERAS memakai kata informal, sapaan akrab, atau partikel santai \
                 (aku, kamu, gak, nggak, udah, gimana, banget, aja, sih, dong, kok). \
                 Contoh patokan: \"I don't know why you didn't tell me.\" -> \"Saya tidak mengetahui alasan Anda tidak memberitahukannya kepada saya.\""
            }
            Tone::Casual => {
                "Gaya CASUAL (Santai Percakapan Harian): WAJIB gunakan pronomina akrab (aku, kamu) serta diksi lisan santai yang wajar \
                 (gak/nggak, udah, gimana, kenapa, banget, aja, bilang, dan partikel ringan seperti sih/kok bila alami). \
                 DILARANG KERAS memakai kata resmi/kaku (Anda, tersebut, apakah, hendak, mengapa, bagaimana). \
                 Contoh patokan: \"I don't know why you didn't tell me.\" -> \"Aku gak tahu kenapa kamu gak bilang ke aku.\""
            }
        };

        let source_clause = if source_lang == "auto" {
            "Deteksi bahasa teks sumber secara otomatis lalu terjemahkan ke Bahasa Indonesia."
        } else {
            "Terjemahkan teks dari bahasa sumber berikut ke Bahasa Indonesia."
        };

        format!(
            "Kamu adalah penerjemah profesional ({source_label} -> Bahasa Indonesia).\n\
             {source_clause}\n\
             {tone_guide}\n\
             Pertahankan struktur paragraf dan baris asli.\n\
             Keluarkan HANYA hasil terjemahan Bahasa Indonesia tanpa komentar tambahan, tanpa penjelasan, dan tanpa membungkus dengan tanda kutip."
        )
    } else if target_lang == "en" {
        let tone_guide = match tone {
            Tone::Natural => {
                "NATURAL tone (Auto-adaptive like DeepL): Automatically match the register and voice of the source text — \
                 keep literary or narrative prose vivid and smooth, render quoted dialogue naturally with realistic contractions where fitting, \
                 and keep formal/technical sections clear and accurate.\n\
                 Reference example: \"Aku tidak tahu kenapa dia belum datang.\" -> \"I don't know why she hasn't arrived yet.\""
            }
            Tone::Formal => {
                "FORMAL tone (Official & Academic English): Strictly use formal vocabulary, full uncontracted verb forms \
                 (do not, cannot, would not), and polite objective phrasing suitable for business or official correspondence. \
                 Forbid slang, casual fillers, or contractions.\n\
                 Reference example: \"Tolong kabari saya kalau berkasnya sudah siap.\" -> \"Please inform me once the documents are ready.\""
            }
            Tone::Casual => {
                "CASUAL tone (Everyday Conversational English): Use relaxed spoken English, everyday idioms, and natural contractions \
                 (don't, didn't, gonna, I'm) as in friendly daily conversation. Avoid stiff bureaucratic words.\n\
                 Reference example: \"Saya tidak tahu kenapa kamu tidak bilang dari tadi.\" -> \"I didn't know why you didn't tell me earlier.\""
            }
        };

        format!(
            "You are a professional translator translating from {source_label} into English.\n\
             {tone_guide}\n\
             Preserve the original paragraph and line-break structure.\n\
             Output ONLY the English translation with no commentary, no explanations, and no wrapping quotes."
        )
    } else {
        let info = language_info(target_lang);
        let tone_guide = match tone {
            Tone::Natural => format!(
                "NATURAL tone (Auto-adaptive): Automatically mirror the tone and register of the source passage in {} ({}) — \
                 flowing narrative for stories/novels, natural dialogue inside quotes, and clear informative style for nonfiction.",
                info.name, info.native_name
            ),
            Tone::Formal => format!(
                "FORMAL tone: Strictly use polite, formal/standard register and honorifics appropriate in {} ({}); avoid slang or casual contractions.",
                info.name, info.native_name
            ),
            Tone::Casual => format!(
                "CASUAL tone: Use relaxed everyday spoken {} ({}) suitable for friends and informal chat; avoid stiff bureaucratic phrasing.",
                info.name, info.native_name
            ),
        };

        format!(
            "You are a professional translator translating from {source_label} into {} ({}).\n\
             {tone_guide}\n\
             Preserve the original paragraph and line-break structure.\n\
             Output ONLY the {} translation with no commentary, no notes, and no wrapping quotes.",
            info.name, info.native_name, info.name
        )
    };

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system,
        },
        ChatMessage {
            role: "user".to_string(),
            content: source_chunk.to_string(),
        },
    ]
}

pub fn build_word_alternatives_messages(
    word: &str,
    sentence_context: &str,
    source_text: &str,
    target_lang: &str,
) -> Vec<ChatMessage> {
    let source_excerpt: String = source_text.chars().take(1000).collect();

    if target_lang == "id" {
        let system = "Kamu adalah editor leksikal Bahasa Indonesia. Berikan 3 sampai 4 alternatif sinonim kata Bahasa Indonesia yang paling kontekstual untuk kata yang dipilih dalam kalimat terjemahan.\n\
             Jaga agar setiap field tetap ringkas: `context_summary` maksimal 8 kata, dan tiap `nuance` maksimal 12 kata.\n\
             Jawab HANYA dengan JSON ringkas satu baris (tanpa spasi/baris baru berlebih) sesuai skema.";
        let user = format!(
            "Kata yang diklik: \"{word}\"\n\
             Kalimat terjemahan: \"{sentence_context}\"\n\
             Teks sumber (rujukan konteks): \"{source_excerpt}\"\n\n\
             Tandai tepat satu opsi terbaik dengan \"recommended\": true."
        );

        vec![
            ChatMessage {
                role: "system".to_string(),
                content: system.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user,
            },
        ]
    } else {
        let info = language_info(target_lang);
        let system = format!(
            "You are a lexical editor for {} ({}). Provide 3 to 4 contextual {} synonyms for the selected word inside the translated sentence.\n\
             Keep each field concise: `context_summary` <= 8 words, each `nuance` (brief meaning/tone difference) <= 12 words.\n\
             Reply ONLY with compact single-line JSON matching the schema.",
            info.name, info.native_name, info.name
        );
        let user = format!(
            "Clicked word ({}): \"{word}\"\n\
             Translated sentence: \"{sentence_context}\"\n\
             Original source excerpt: \"{source_excerpt}\"\n\n\
             Mark exactly one best option with \"recommended\": true.",
            info.name
        );

        vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user,
            },
        ]
    }
}

pub fn word_alternatives_json_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "word_alternatives",
            "strict": true,
            "schema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["context_summary", "alternatives"],
                "properties": {
                    "context_summary": { "type": "string" },
                    "alternatives": {
                        "type": "array",
                        "minItems": 2,
                        "maxItems": 5,
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["word", "nuance", "recommended"],
                            "properties": {
                                "word": { "type": "string" },
                                "nuance": { "type": "string" },
                                "recommended": { "type": "boolean" }
                            }
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_tones_enforce_distinct_indonesian_and_english_registers() {
        let id_formal =
            &build_text_translation_messages("Hello", "en", "id", Tone::Formal)[0].content;
        let id_natural =
            &build_text_translation_messages("Hello", "en", "id", Tone::Natural)[0].content;
        let id_casual =
            &build_text_translation_messages("Hello", "en", "id", Tone::Casual)[0].content;

        assert_ne!(id_formal, id_natural);
        assert_ne!(id_formal, id_casual);
        assert_ne!(id_natural, id_casual);
        assert!(id_natural.contains("Adaptif seperti DeepL"));
        assert!(id_formal.contains("Gaya FORMAL"));
        assert!(id_casual.contains("Gaya CASUAL"));

        let en_formal =
            &build_text_translation_messages("Halo", "id", "en", Tone::Formal)[0].content;
        let en_natural =
            &build_text_translation_messages("Halo", "id", "en", Tone::Natural)[0].content;
        let en_casual =
            &build_text_translation_messages("Halo", "id", "en", Tone::Casual)[0].content;
        assert!(en_natural.contains("Auto-adaptive like DeepL"));
        assert!(en_formal.contains("FORMAL tone"));
        assert!(en_casual.contains("CASUAL tone"));

        let auto_ja =
            &build_text_translation_messages("Hello", "auto", "ja", Tone::Natural)[0].content;
        assert!(auto_ja.contains("automatically detected source language"));
        assert!(auto_ja.contains("Japanese"));
    }

    #[test]
    fn subtitle_genres_differentiate_film_doc_and_languages() {
        let film_id = Genre::Film.prompt_instruction_for_lang("id");
        let doc_id = Genre::Documentary.prompt_instruction_for_lang("id");
        let film_en = Genre::Film.prompt_instruction_for_lang("en");
        let anime_ja = Genre::Anime.prompt_instruction_for_lang("ja");

        assert!(film_id.contains("aku/kamu, namaku"));
        assert!(doc_id.contains("EYD") && doc_id.contains("DILARANG"));
        assert!(film_en.contains("Cinema & TV streaming"));
        assert!(anime_ja.contains("Japanese"));
        assert_ne!(film_id, doc_id);
    }

    #[test]
    fn word_alternatives_prompt_targets_requested_language_without_antonyms() {
        let alt_id = &build_word_alternatives_messages("bertemu", "aku bertemu kamu", "I met you", "id")[0].content;
        let alt_en = &build_word_alternatives_messages("met", "I met you", "Aku bertemu kamu", "en")[0].content;

        assert!(alt_id.contains("Bahasa Indonesia"));
        assert!(alt_en.contains("English"));
        assert!(!alt_id.to_lowercase().contains("antonim"));
        assert!(!alt_en.to_lowercase().contains("antonym"));
    }
}
