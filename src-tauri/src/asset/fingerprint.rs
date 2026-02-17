use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::project::model::Fingerprint;

pub fn compute_file_fingerprint(path: &Path) -> Result<Fingerprint, String> {
    let bytes = fs::read(path).map_err(|e| format!("读取文件失败 {}: {}", path.display(), e))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hasher.finalize();
    let hex = format!("{:x}", hash);

    Ok(Fingerprint {
        algo: "sha256".to_string(),
        value: format!("sha256:{}", hex),
        basis: "file_bytes".to_string(),
    })
}
