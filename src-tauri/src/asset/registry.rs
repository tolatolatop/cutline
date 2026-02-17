use crate::project::model::Asset;

pub fn find_duplicate<'a>(assets: &'a [Asset], fingerprint_value: &str) -> Option<&'a Asset> {
    assets
        .iter()
        .find(|a| a.fingerprint.value == fingerprint_value)
}
