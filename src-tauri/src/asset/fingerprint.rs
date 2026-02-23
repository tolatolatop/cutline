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

pub fn compute_content_fingerprint(content: &[u8]) -> Fingerprint {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let hash = hasher.finalize();
    let hex = format!("{:x}", hash);

    Fingerprint {
        algo: "sha256".to_string(),
        value: format!("sha256:{}", hex),
        basis: "content_json".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_fingerprint_nonexistent_path_returns_error() {
        let result = compute_file_fingerprint(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn file_fingerprint_valid_file() {
        let dir = std::env::temp_dir().join("cutline_fp_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.txt");
        std::fs::write(&path, b"hello world").unwrap();

        let fp = compute_file_fingerprint(&path).unwrap();
        assert_eq!(fp.algo, "sha256");
        assert!(fp.value.starts_with("sha256:"));
        assert_eq!(fp.basis, "file_bytes");
        assert_eq!(fp.value.len(), 7 + 64); // "sha256:" + 64 hex chars

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn content_fingerprint_deterministic() {
        let fp1 = compute_content_fingerprint(b"test content");
        let fp2 = compute_content_fingerprint(b"test content");
        assert_eq!(fp1.value, fp2.value);
    }

    #[test]
    fn content_fingerprint_different_for_different_content() {
        let fp1 = compute_content_fingerprint(b"content A");
        let fp2 = compute_content_fingerprint(b"content B");
        assert_ne!(fp1.value, fp2.value);
    }

    #[test]
    fn content_fingerprint_fields() {
        let fp = compute_content_fingerprint(b"hello");
        assert_eq!(fp.algo, "sha256");
        assert_eq!(fp.basis, "content_json");
        assert!(fp.value.starts_with("sha256:"));
        assert_eq!(fp.value.len(), 7 + 64);
    }

    #[test]
    fn content_fingerprint_empty_input() {
        let fp = compute_content_fingerprint(b"");
        assert_eq!(fp.algo, "sha256");
        assert!(!fp.value.is_empty());
    }

    #[test]
    fn content_fingerprint_utf8_content() {
        let fp = compute_content_fingerprint("一只可爱的猫咪".as_bytes());
        assert_eq!(fp.algo, "sha256");
        assert_eq!(fp.value.len(), 7 + 64);
    }
}
