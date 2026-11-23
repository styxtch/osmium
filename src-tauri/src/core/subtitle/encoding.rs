use chardetng::EncodingDetector;
use encoding_rs::{UTF_16BE, UTF_16LE, UTF_8};

use crate::error::{AppError, AppResult};

/// Result of decoding raw subtitle file bytes into normalized UTF-8 text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSubtitleText {
    pub text: String,
    pub detected_encoding: String,
    pub had_bom: bool,
}

/// Decodes raw subtitle bytes into normalized UTF-8 (`\n` line endings).
///
/// Detection order:
/// 1. Explicit BOM (`UTF-8`, `UTF-16 LE`, `UTF-16 BE`)
/// 2. Strict UTF-8 validation
/// 3. `chardetng` statistical heuristic + `encoding_rs` transcoding
pub fn decode_subtitle_bytes(bytes: &[u8]) -> AppResult<DecodedSubtitleText> {
    if bytes.is_empty() {
        return Err(AppError::InvalidInput(
            "Subtitle file is empty.".to_string(),
        ));
    }

    // 1. Check explicit BOM headers first.
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let stripped = &bytes[3..];
        let text = match std::str::from_utf8(stripped) {
            Ok(s) => s.to_string(),
            Err(_) => {
                let (cow, _) = UTF_8.decode_without_bom_handling(stripped);
                cow.into_owned()
            }
        };
        return Ok(DecodedSubtitleText {
            text: normalize_line_endings(&text),
            detected_encoding: "UTF-8-BOM".to_string(),
            had_bom: true,
        });
    }

    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (cow, _) = UTF_16LE.decode_without_bom_handling(&bytes[2..]);
        return Ok(DecodedSubtitleText {
            text: normalize_line_endings(&cow),
            detected_encoding: "UTF-16LE".to_string(),
            had_bom: true,
        });
    }

    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _) = UTF_16BE.decode_without_bom_handling(&bytes[2..]);
        return Ok(DecodedSubtitleText {
            text: normalize_line_endings(&cow),
            detected_encoding: "UTF-16BE".to_string(),
            had_bom: true,
        });
    }

    // 2. Prefer strict UTF-8 when valid (avoids false positives on mostly ASCII text).
    if let Ok(utf8_str) = std::str::from_utf8(bytes) {
        return Ok(DecodedSubtitleText {
            text: normalize_line_endings(utf8_str),
            detected_encoding: "UTF-8".to_string(),
            had_bom: false,
        });
    }

    // 3. Fallback to chardetng + encoding_rs for legacy codepages (Windows-1252, Shift_JIS, etc.).
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);
    let (decoded, used_encoding, _had_errors) = encoding.decode(bytes);

    Ok(DecodedSubtitleText {
        text: normalize_line_endings(&decoded),
        detected_encoding: used_encoding.name().to_string(),
        had_bom: false,
    })
}

fn normalize_line_endings(input: &str) -> String {
    if !input.contains('\r') {
        return input.to_string();
    }
    input.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf8_bom_and_normalizes_crlf() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"1\r\n00:00:01,000 --> 00:00:02,000\r\nHello world\r\n");

        let decoded = decode_subtitle_bytes(&bytes).expect("decode utf-8 bom");
        assert!(decoded.had_bom);
        assert_eq!(decoded.detected_encoding, "UTF-8-BOM");
        assert!(!decoded.text.contains('\r'));
        assert!(decoded.text.starts_with("1\n00:00:01,000"));
    }

    #[test]
    fn decodes_utf16le_bom() {
        let sample = "1\r\n00:00:01,000 --> 00:00:02,000\r\nHalo Dunia";
        let mut bytes = vec![0xFF, 0xFE];
        for unit in sample.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }

        let decoded = decode_subtitle_bytes(&bytes).expect("decode utf-16le");
        assert!(decoded.had_bom);
        assert_eq!(decoded.detected_encoding, "UTF-16LE");
        assert_eq!(
            decoded.text,
            "1\n00:00:01,000 --> 00:00:02,000\nHalo Dunia"
        );
    }

    #[test]
    fn decodes_legacy_windows_1252_and_shift_jis() {
        // Windows-1252 smart quote + accented vowel: "Café — Test"
        let win1252: &[u8] = &[
            b'C', b'a', b'f', 0xE9, b' ', 0x97, b' ', b'T', b'e', b's', b't',
        ];
        let decoded_win = decode_subtitle_bytes(win1252).expect("decode windows-1252");
        assert!(!decoded_win.had_bom);
        assert!(decoded_win.text.contains("Caf"));

        // Shift_JIS encoded Japanese subtitle text ("こんにちは世界")
        let (sjis_bytes, _, _) = encoding_rs::SHIFT_JIS.encode("こんにちは世界");
        let decoded_sjis = decode_subtitle_bytes(&sjis_bytes).expect("decode shift-jis");
        assert_eq!(decoded_sjis.text, "こんにちは世界");
    }
}
