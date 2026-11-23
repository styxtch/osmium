use crate::core::engine::llama_client::ChatMessage;
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
}

pub fn build_text_translation_messages(source_chunk: &str, tone: Tone) -> Vec<ChatMessage> {
    let tone_guide = match tone {
        Tone::Natural => {
            "Gaya NATURAL: Terjemahan harus mengalir idiomatis dan seimbang sesuai nuansa teks asli, tidak kaku seperti dokumen hukum dan tidak terlalu gaul. \
             Sesuaikan sapaan secara wajar mengikuti konteks kalimat sumber. \
             Contoh patokan: \"I don't know why you didn't tell me.\" -> \"Aku tidak tahu kenapa kamu tidak bilang padaku.\""
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

    let system = format!(
        "Kamu adalah penerjemah profesional Inggris ke Bahasa Indonesia.\n\
         {tone_guide}\n\
         Pertahankan struktur paragraf dan baris asli.\n\
         Keluarkan HANYA hasil terjemahan Bahasa Indonesia tanpa komentar tambahan, tanpa penjelasan, dan tanpa membungkus dengan tanda kutip."
    );

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
) -> Vec<ChatMessage> {
    let source_excerpt: String = source_text.chars().take(1000).collect();
    let system = "Kamu adalah editor leksikal Bahasa Indonesia. Berikan 3 sampai 4 alternatif sinonim kata yang paling kontekstual untuk kata yang dipilih dalam kalimat terjemahan dalam Bahasa Indonesia.\n\
         Jaga agar setiap field tetap ringkas: `context_summary` maksimal 8 kata, dan tiap `nuance` maksimal 12 kata.\n\
         Jawab HANYA dengan JSON ringkas satu baris (tanpa spasi/baris baru berlebih) sesuai skema.";
    let user = format!(
        "Kata yang diklik: \"{word}\"\n\
         Kalimat terjemahan: \"{sentence_context}\"\n\
         Teks sumber Inggris (rujukan konteks): \"{source_excerpt}\"\n\n\
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
    fn text_tones_enforce_distinct_indonesian_registers() {
        let formal = &build_text_translation_messages("Hello", Tone::Formal)[0].content;
        let natural = &build_text_translation_messages("Hello", Tone::Natural)[0].content;
        let casual = &build_text_translation_messages("Hello", Tone::Casual)[0].content;

        assert_ne!(formal, natural);
        assert_ne!(formal, casual);
        assert_ne!(natural, casual);

        assert!(formal.contains("Gaya FORMAL"));
        assert!(formal.contains("DILARANG KERAS memakai kata informal"));
        assert!(casual.contains("Gaya CASUAL"));
        assert!(casual.contains("DILARANG KERAS memakai kata resmi/kaku"));
        assert!(natural.contains("Gaya NATURAL"));
    }

    #[test]
    fn subtitle_genres_differentiate_film_and_documentary_rules() {
        let film = Genre::Film.prompt_instruction();
        let doc = Genre::Documentary.prompt_instruction();
        let anime = Genre::Anime.prompt_instruction();
        let gaming = Genre::Gaming.prompt_instruction();

        assert!(film.contains("aku/kamu, namaku"));
        assert!(doc.contains("EYD") && doc.contains("DILARANG"));
        assert!(anime.contains("senpai"));
        assert!(gaming.contains("quest"));
        assert_ne!(film, doc);
    }
}
