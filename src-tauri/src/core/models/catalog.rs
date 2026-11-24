use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogModel {
    pub id: String,
    pub display_name: String,
    pub tier: String,
    pub quantization: String,
    pub repo_id: String,
    pub commit_sha: String,
    pub file_name: String,
    pub download_url: String,
    pub file_size_bytes: u64,
    pub estimated_vram_mb: u64,
    pub sha256: String,
    pub block_count: u32,
    pub description: String,
    pub badge_label: String,
}

pub fn official_catalog() -> Vec<CatalogModel> {
    vec![
        CatalogModel {
            id: "qwen3-8b-q4km".to_string(),
            display_name: "Qwen 3 8B".to_string(),
            tier: "optimal_8b".to_string(),
            quantization: "Q4_K_M".to_string(),
            repo_id: "Qwen/Qwen3-8B-GGUF".to_string(),
            commit_sha: "7c41481f57cb95916b40956ab2f0b139b296d974".to_string(),
            file_name: "Qwen3-8B-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen3-8B-GGUF/resolve/7c41481f57cb95916b40956ab2f0b139b296d974/Qwen3-8B-Q4_K_M.gguf".to_string(),
            file_size_bytes: 5_027_783_488,
            estimated_vram_mb: 6200,
            sha256: "d98cdcbd03e17ce47681435b5150e34c1417f50b5c0019dd560e4882c5745785".to_string(),
            block_count: 36,
            description: "Highest quality for idioms, literature, and film/anime dialogue. Optimal for 8–12+ GB VRAM GPUs.".to_string(),
            badge_label: "Recommended 8GB+ GPU".to_string(),
        },
        CatalogModel {
            id: "qwen3-4b-q4km".to_string(),
            display_name: "Qwen 3 4B".to_string(),
            tier: "balanced_4b".to_string(),
            quantization: "Q4_K_M".to_string(),
            repo_id: "Qwen/Qwen3-4B-GGUF".to_string(),
            commit_sha: "bc640142c66e1fdd12af0bd68f40445458f3869b".to_string(),
            file_name: "Qwen3-4B-Q4_K_M.gguf".to_string(),
            download_url: "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/bc640142c66e1fdd12af0bd68f40445458f3869b/Qwen3-4B-Q4_K_M.gguf".to_string(),
            file_size_bytes: 2_497_280_256,
            estimated_vram_mb: 3600,
            sha256: "7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5".to_string(),
            block_count: 36,
            description: "Ultra-low VRAM footprint with high throughput and fast response. Ideal for 6 GB GPUs or CPU fallback.".to_string(),
            badge_label: "VRAM Saver (4–6GB GPU)".to_string(),
        },
    ]
}

pub fn find_catalog_model(id: &str) -> Option<CatalogModel> {
    official_catalog().into_iter().find(|m| m.id == id)
}
