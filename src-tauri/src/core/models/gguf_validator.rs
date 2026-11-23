use crate::error::{AppError, AppResult};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const MAX_KV_INSPECT: u64 = 2048;
const MAX_STRING_LEN: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GgufMetadata {
    pub version: u32,
    pub architecture: Option<String>,
    pub display_name: String,
    pub quantization: Option<String>,
    pub file_size_bytes: u64,
    pub block_count: Option<u32>,
    pub estimated_vram_mb: u64,
}

pub fn validate_gguf_file(path: &Path) -> AppResult<GgufMetadata> {
    let file = File::open(path)?;
    let file_size_bytes = file.metadata()?.len();
    if file_size_bytes < 24 {
        return Err(AppError::InvalidInput(
            "File is too small and is not a valid .gguf model".to_string(),
        ));
    }

    let mut reader = BufReader::new(file);
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != GGUF_MAGIC {
        return Err(AppError::InvalidInput(
            "GGUF magic bytes not found. Ensure the selected file is a valid .gguf model."
                .to_string(),
        ));
    }

    let version = read_u32(&mut reader)?;
    if version != 2 && version != 3 {
        return Err(AppError::InvalidInput(format!(
            "Unsupported GGUF version ({version}). Osmium supports GGUF v2 and v3."
        )));
    }

    let _tensor_count = read_u64(&mut reader)?;
    let metadata_kv_count = read_u64(&mut reader)?;

    let mut architecture: Option<String> = None;
    let mut model_name: Option<String> = None;
    let mut block_count: Option<u32> = None;

    let inspect_count = metadata_kv_count.min(MAX_KV_INSPECT);
    for _ in 0..inspect_count {
        let key = match read_gguf_string(&mut reader) {
            Ok(k) => k,
            Err(_) => break,
        };
        let value_type = match read_u32(&mut reader) {
            Ok(t) => t,
            Err(_) => break,
        };

        if key == "general.architecture" && value_type == 8 {
            if let Ok(val) = read_gguf_string(&mut reader) {
                architecture = Some(val);
            } else {
                break;
            }
        } else if key == "general.name" && value_type == 8 {
            if let Ok(val) = read_gguf_string(&mut reader) {
                if !val.trim().is_empty() {
                    model_name = Some(val);
                }
            } else {
                break;
            }
        } else if key.ends_with(".block_count") {
            match read_gguf_int_as_u32(&mut reader, value_type) {
                Ok(Some(n)) => block_count = Some(n),
                Ok(None) => {
                    if skip_gguf_value(&mut reader, value_type, 0).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        } else if skip_gguf_value(&mut reader, value_type, 0).is_err() {
            break;
        }
    }

    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Custom GGUF")
        .to_string();
    let display_name = model_name.unwrap_or_else(|| file_stem.clone());
    let quantization = infer_quantization_from_name(&file_stem);
    let model_mb = (file_size_bytes as f64) / (1024.0 * 1024.0);
    let estimated_vram_mb = (model_mb * 1.15).ceil() as u64 + 350;

    Ok(GgufMetadata {
        version,
        architecture,
        display_name,
        quantization,
        file_size_bytes,
        block_count,
        estimated_vram_mb,
    })
}

fn infer_quantization_from_name(name: &str) -> Option<String> {
    let upper = name.to_uppercase();
    let known = [
        "Q4_K_M", "Q4_K_S", "Q5_K_M", "Q5_K_S", "Q6_K", "Q8_0", "Q5_0", "Q4_0", "Q3_K_M",
        "Q3_K_L", "Q2_K", "IQ4_XS", "IQ4_NL", "BF16", "F16",
    ];
    known
        .into_iter()
        .find(|pat| upper.contains(pat))
        .map(str::to_string)
}

fn read_u32<R: Read>(r: &mut R) -> AppResult<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64<R: Read>(r: &mut R) -> AppResult<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_gguf_string<R: Read>(r: &mut R) -> AppResult<String> {
    let len = read_u64(r)?;
    if len > MAX_STRING_LEN {
        return Err(AppError::InvalidInput("GGUF metadata string is too large".to_string()));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn read_gguf_int_as_u32<R: Read>(r: &mut R, value_type: u32) -> AppResult<Option<u32>> {
    match value_type {
        4 => Ok(Some(read_u32(r)?)),
        5 => {
            let mut buf = [0u8; 4];
            r.read_exact(&mut buf)?;
            let v = i32::from_le_bytes(buf);
            Ok((v >= 0).then_some(v as u32))
        }
        10 => {
            let v = read_u64(r)?;
            Ok(u32::try_from(v).ok())
        }
        11 => {
            let mut buf = [0u8; 8];
            r.read_exact(&mut buf)?;
            let v = i64::from_le_bytes(buf);
            Ok(u32::try_from(v).ok())
        }
        _ => Ok(None),
    }
}

fn scalar_size(value_type: u32) -> Option<i64> {
    match value_type {
        0 | 1 | 7 => Some(1),  // UINT8, INT8, BOOL
        2 | 3 => Some(2),      // UINT16, INT16
        4 | 5 | 6 => Some(4),  // UINT32, INT32, FLOAT32
        10 | 11 | 12 => Some(8), // UINT64, INT64, FLOAT64
        _ => None,
    }
}

fn skip_gguf_value<R: Read + Seek>(r: &mut R, value_type: u32, depth: u8) -> AppResult<()> {
    if depth > 4 {
        return Err(AppError::InvalidInput("GGUF nested array is too deep".to_string()));
    }
    if let Some(sz) = scalar_size(value_type) {
        r.seek(SeekFrom::Current(sz))?;
        return Ok(());
    }
    match value_type {
        8 => {
            let len = read_u64(r)?;
            if len > MAX_STRING_LEN * 16 {
                return Err(AppError::InvalidInput("GGUF string is too long".to_string()));
            }
            r.seek(SeekFrom::Current(len as i64))?;
            Ok(())
        }
        9 => {
            let elem_type = read_u32(r)?;
            let count = read_u64(r)?;
            if let Some(sz) = scalar_size(elem_type) {
                let total = count.saturating_mul(sz as u64);
                r.seek(SeekFrom::Current(total as i64))?;
            } else {
                for _ in 0..count {
                    skip_gguf_value(r, elem_type, depth + 1)?;
                }
            }
            Ok(())
        }
        other => Err(AppError::InvalidInput(format!(
            "Unknown GGUF value type: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_gguf_str(buf: &mut Vec<u8>, s: &str) {
        buf.extend_from_slice(&(s.len() as u64).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    #[test]
    fn validates_synthetic_gguf_v3_and_extracts_metadata() {
        let mut data = Vec::new();
        data.extend_from_slice(b"GGUF");
        data.extend_from_slice(&3u32.to_le_bytes()); // version 3
        data.extend_from_slice(&12u64.to_le_bytes()); // tensor_count
        data.extend_from_slice(&4u64.to_le_bytes()); // metadata_kv_count

        // 1: general.architecture = "qwen3"
        write_gguf_str(&mut data, "general.architecture");
        data.extend_from_slice(&8u32.to_le_bytes());
        write_gguf_str(&mut data, "qwen3");

        // 2: tokenizer array of strings (skip test)
        write_gguf_str(&mut data, "tokenizer.ggml.tokens");
        data.extend_from_slice(&9u32.to_le_bytes());
        data.extend_from_slice(&8u32.to_le_bytes());
        data.extend_from_slice(&2u64.to_le_bytes());
        write_gguf_str(&mut data, "<s>");
        write_gguf_str(&mut data, "</s>");

        // 3: general.name = "My FineTune 7B"
        write_gguf_str(&mut data, "general.name");
        data.extend_from_slice(&8u32.to_le_bytes());
        write_gguf_str(&mut data, "My FineTune 7B");

        // 4: qwen3.block_count = 36
        write_gguf_str(&mut data, "qwen3.block_count");
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(&36u32.to_le_bytes());

        let path = std::env::temp_dir().join(format!("osmium_valid_{}.gguf", uuid::Uuid::new_v4()));
        let mut f = File::create(&path).unwrap();
        f.write_all(&data).unwrap();
        drop(f);

        let meta = validate_gguf_file(&path).expect("valid synthetic GGUF");
        let _ = std::fs::remove_file(&path);

        assert_eq!(meta.version, 3);
        assert_eq!(meta.architecture.as_deref(), Some("qwen3"));
        assert_eq!(meta.display_name, "My FineTune 7B");
        assert_eq!(meta.block_count, Some(36));
    }

    #[test]
    fn rejects_invalid_magic_and_unsupported_version() {
        let bad_magic = std::env::temp_dir().join(format!("osmium_bad_{}.gguf", uuid::Uuid::new_v4()));
        std::fs::write(&bad_magic, b"NOTGGUF_01234567890123456789").unwrap();
        assert!(validate_gguf_file(&bad_magic).is_err());
        let _ = std::fs::remove_file(&bad_magic);

        let mut v1_data = Vec::new();
        v1_data.extend_from_slice(b"GGUF");
        v1_data.extend_from_slice(&1u32.to_le_bytes());
        v1_data.extend_from_slice(&0u64.to_le_bytes());
        v1_data.extend_from_slice(&0u64.to_le_bytes());
        let v1_path = std::env::temp_dir().join(format!("osmium_v1_{}.gguf", uuid::Uuid::new_v4()));
        std::fs::write(&v1_path, &v1_data).unwrap();
        assert!(validate_gguf_file(&v1_path).is_err());
        let _ = std::fs::remove_file(&v1_path);
    }
}
