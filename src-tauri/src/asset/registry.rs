use crate::project::model::Asset;

pub fn find_duplicate<'a>(assets: &'a [Asset], fingerprint_value: &str) -> Option<&'a Asset> {
    assets
        .iter()
        .find(|a| a.fingerprint.value == fingerprint_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::Fingerprint;

    fn make_asset(id: &str, fp_value: &str, asset_type: &str) -> Asset {
        Asset {
            asset_id: id.to_string(),
            asset_type: asset_type.to_string(),
            source: "authored".to_string(),
            fingerprint: Fingerprint {
                algo: "sha256".to_string(),
                value: fp_value.to_string(),
                basis: "content_json".to_string(),
            },
            path: format!("workspace/assets/prompts/{}.md", id),
            meta: serde_json::json!({"kind": asset_type}),
            generation: None,
            tags: vec![],
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn find_duplicate_returns_matching_asset() {
        let assets = vec![
            make_asset("a1", "sha256:aaa", "prompt"),
            make_asset("a2", "sha256:bbb", "video"),
        ];
        let found = find_duplicate(&assets, "sha256:aaa");
        assert!(found.is_some());
        assert_eq!(found.unwrap().asset_id, "a1");
    }

    #[test]
    fn find_duplicate_returns_none_when_no_match() {
        let assets = vec![make_asset("a1", "sha256:aaa", "prompt")];
        assert!(find_duplicate(&assets, "sha256:zzz").is_none());
    }

    #[test]
    fn find_duplicate_empty_list() {
        let assets: Vec<Asset> = vec![];
        assert!(find_duplicate(&assets, "sha256:aaa").is_none());
    }
}
