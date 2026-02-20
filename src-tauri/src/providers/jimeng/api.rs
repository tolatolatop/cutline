use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::client::JimengClient;
use super::constants::{get_aspect_ratio, resolve_model, DRAFT_VERSION};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResult {
    pub history_id: String,
    pub submit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusResult {
    pub status: u32,
    pub fail_code: String,
    pub item_list: Vec<TaskItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskItem {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditInfo {
    pub gift_credit: f64,
    pub purchase_credit: f64,
    pub vip_credit: f64,
}

// ---------------------------------------------------------------------------
// draft_content builder
// ---------------------------------------------------------------------------

fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn random_seed() -> u64 {
    rand::thread_rng().gen_range(2_500_000_000u64..2_600_000_000u64)
}

pub(crate) fn build_txt2img_draft(
    prompt: &str,
    model: &str,
    ratio: &str,
    negative_prompt: &str,
    seed: Option<u64>,
    sample_strength: f64,
) -> String {
    let aspect = get_aspect_ratio(ratio);
    let size = aspect.size_2k;
    let seed = seed.unwrap_or_else(random_seed);

    let component_id = new_uuid();

    let draft = json!({
        "type": "draft",
        "id": new_uuid(),
        "min_version": DRAFT_VERSION,
        "min_features": [],
        "is_from_tsn": true,
        "version": DRAFT_VERSION,
        "main_component_id": component_id,
        "component_list": [{
            "type": "image_base_component",
            "id": component_id,
            "min_version": DRAFT_VERSION,
            "generate_type": "generate",
            "aigc_mode": "workbench",
            "abilities": {
                "type": "",
                "id": new_uuid(),
                "generate": {
                    "type": "",
                    "id": new_uuid(),
                    "core_param": {
                        "type": "",
                        "id": new_uuid(),
                        "model": model,
                        "prompt": prompt,
                        "negative_prompt": negative_prompt,
                        "seed": seed,
                        "sample_strength": sample_strength,
                        "image_ratio": aspect.ratio_type,
                        "intelligent_ratio": false,
                        "large_image_info": {
                            "type": "",
                            "id": new_uuid(),
                            "height": size.height,
                            "width": size.width,
                            "resolution_type": "2k"
                        }
                    },
                    "history_option": {
                        "type": "",
                        "id": new_uuid()
                    }
                }
            }
        }]
    });

    draft.to_string()
}

pub(crate) fn build_metrics_extra(
    prompt: &str,
    model: &str,
    image_count: u32,
    image_ratio: u32,
    negative_prompt: &str,
) -> String {
    let mut data = json!({
        "seed": random_seed(),
        "prompt": prompt,
        "image_count": image_count,
        "image_ratio": image_ratio,
        "model": model,
        "draft_version": DRAFT_VERSION,
        "mode": "generate"
    });

    if !negative_prompt.is_empty() {
        data["negative_prompt"] = json!(negative_prompt);
    }

    data.to_string()
}

// ---------------------------------------------------------------------------
// Response parsing helpers (extracted for testability)
// ---------------------------------------------------------------------------

fn parse_history_id(resp: &Value) -> String {
    resp.pointer("/data/aigc_data/history_record_id")
        .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn parse_credit_response(resp: &Value) -> Result<CreditInfo, String> {
    let credit = resp
        .pointer("/data/credit")
        .ok_or("Missing /data/credit in response")?;

    Ok(CreditInfo {
        gift_credit: credit.get("gift_credit").and_then(|v| v.as_f64()).unwrap_or(0.0),
        purchase_credit: credit.get("purchase_credit").and_then(|v| v.as_f64()).unwrap_or(0.0),
        vip_credit: credit.get("vip_credit").and_then(|v| v.as_f64()).unwrap_or(0.0),
    })
}

// ---------------------------------------------------------------------------
// API paths
// ---------------------------------------------------------------------------

const GENERATE_PATH: &str = "/mweb/v1/aigc_draft/generate";
const HISTORY_PATH: &str = "/mweb/v1/get_history_by_ids";
const CREDIT_PATH: &str = "/commerce/v1/benefits/user_credit";
const CREDIT_REFERER: &str = "https://jimeng.jianying.com/ai-tool/image/generate";

// ---------------------------------------------------------------------------
// AIGC API
// ---------------------------------------------------------------------------

pub async fn generate_image(
    client: &JimengClient,
    prompt: &str,
    model: &str,
    ratio: &str,
    negative_prompt: &str,
    image_count: u32,
) -> Result<GenerateResult, String> {
    let internal_model = resolve_model(model);
    let aspect = get_aspect_ratio(ratio);

    let draft = build_txt2img_draft(
        prompt,
        &internal_model,
        ratio,
        negative_prompt,
        None,
        0.5,
    );
    let metrics = build_metrics_extra(
        prompt,
        &internal_model,
        image_count,
        aspect.ratio_type,
        negative_prompt,
    );

    let submit_id = new_uuid();

    let body = json!({
        "extend": { "root_model": internal_model },
        "submit_id": submit_id,
        "metrics_extra": metrics,
        "draft_content": draft,
        "http_common_info": { "aid": 513695 }
    });

    let resp = client.post(GENERATE_PATH, &body, &internal_model, false, None).await?;
    let history_id = parse_history_id(&resp);

    Ok(GenerateResult {
        history_id,
        submit_id,
    })
}

// ---------------------------------------------------------------------------
// Task status
// ---------------------------------------------------------------------------

pub async fn get_task_status(
    client: &JimengClient,
    history_ids: &[String],
) -> Result<Value, String> {
    let body = json!({
        "history_ids": history_ids,
        "image_info": {
            "width": 2048,
            "height": 2048,
            "format": "webp",
            "image_scene_list": []
        },
        "http_common_info": { "aid": 513695 }
    });

    client.post(HISTORY_PATH, &body, "", false, None).await
}

// ---------------------------------------------------------------------------
// Credit API
// ---------------------------------------------------------------------------

pub async fn get_credit(client: &JimengClient) -> Result<CreditInfo, String> {
    let extra_headers = [("Referer", CREDIT_REFERER)];

    let resp = client
        .post(CREDIT_PATH, &json!({}), "", false, Some(&extra_headers))
        .await?;

    parse_credit_response(&resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_is_valid_json() {
        let draft = build_txt2img_draft("test prompt", "high_aes_general_v40l", "1:1", "", None, 0.5);
        let parsed: Value = serde_json::from_str(&draft).expect("draft should be valid JSON");
        assert_eq!(parsed["type"], "draft");
    }

    #[test]
    fn draft_has_required_top_level_fields() {
        let draft = build_txt2img_draft("hello", "model_v1", "16:9", "", None, 0.5);
        let v: Value = serde_json::from_str(&draft).unwrap();

        assert_eq!(v["type"], "draft");
        assert!(v["id"].is_string());
        assert_eq!(v["version"], DRAFT_VERSION);
        assert_eq!(v["min_version"], DRAFT_VERSION);
        assert!(v["is_from_tsn"].as_bool().unwrap());
        assert!(v["main_component_id"].is_string());
        assert!(v["component_list"].is_array());
        assert_eq!(v["component_list"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn draft_component_structure() {
        let draft = build_txt2img_draft("cat", "model_v1", "1:1", "ugly", Some(12345), 0.7);
        let v: Value = serde_json::from_str(&draft).unwrap();
        let comp = &v["component_list"][0];

        assert_eq!(comp["type"], "image_base_component");
        assert_eq!(comp["generate_type"], "generate");
        assert_eq!(comp["aigc_mode"], "workbench");

        let core = &comp["abilities"]["generate"]["core_param"];
        assert_eq!(core["model"], "model_v1");
        assert_eq!(core["prompt"], "cat");
        assert_eq!(core["negative_prompt"], "ugly");
        assert_eq!(core["seed"], 12345);
        assert_eq!(core["sample_strength"], 0.7);
        assert_eq!(core["image_ratio"], 1); // 1:1 -> ratio_type 1
        assert_eq!(core["intelligent_ratio"], false);
    }

    #[test]
    fn draft_16_9_aspect_ratio() {
        let draft = build_txt2img_draft("test", "m", "16:9", "", None, 0.5);
        let v: Value = serde_json::from_str(&draft).unwrap();
        let core = &v["component_list"][0]["abilities"]["generate"]["core_param"];

        assert_eq!(core["image_ratio"], 3);
        let large = &core["large_image_info"];
        assert_eq!(large["width"], 2560);
        assert_eq!(large["height"], 1440);
        assert_eq!(large["resolution_type"], "2k");
    }

    #[test]
    fn draft_main_component_id_matches() {
        let draft = build_txt2img_draft("test", "m", "1:1", "", None, 0.5);
        let v: Value = serde_json::from_str(&draft).unwrap();

        let main_id = v["main_component_id"].as_str().unwrap();
        let comp_id = v["component_list"][0]["id"].as_str().unwrap();
        assert_eq!(main_id, comp_id);
    }

    #[test]
    fn draft_uuids_are_unique() {
        let draft = build_txt2img_draft("test", "m", "1:1", "", None, 0.5);
        let v: Value = serde_json::from_str(&draft).unwrap();

        let draft_id = v["id"].as_str().unwrap();
        let main_id = v["main_component_id"].as_str().unwrap();
        assert_ne!(draft_id, main_id, "draft id and main_component_id should differ");
    }

    #[test]
    fn metrics_extra_is_valid_json() {
        let m = build_metrics_extra("test", "model_v1", 4, 1, "");
        let v: Value = serde_json::from_str(&m).expect("metrics should be valid JSON");
        assert_eq!(v["prompt"], "test");
        assert_eq!(v["model"], "model_v1");
        assert_eq!(v["image_count"], 4);
        assert_eq!(v["image_ratio"], 1);
        assert_eq!(v["mode"], "generate");
        assert_eq!(v["draft_version"], DRAFT_VERSION);
        assert!(v["seed"].is_u64());
    }

    #[test]
    fn metrics_extra_with_negative_prompt() {
        let m = build_metrics_extra("cat", "model", 2, 3, "ugly");
        let v: Value = serde_json::from_str(&m).unwrap();
        assert_eq!(v["negative_prompt"], "ugly");
    }

    #[test]
    fn metrics_extra_without_negative_prompt() {
        let m = build_metrics_extra("cat", "model", 2, 3, "");
        let v: Value = serde_json::from_str(&m).unwrap();
        assert!(v.get("negative_prompt").is_none());
    }

    #[test]
    fn generate_result_serialization() {
        let r = GenerateResult {
            history_id: "h123".into(),
            submit_id: "s456".into(),
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["historyId"], "h123");
        assert_eq!(json["submitId"], "s456");
    }

    #[test]
    fn credit_info_serialization() {
        let c = CreditInfo {
            gift_credit: 100.0,
            purchase_credit: 50.5,
            vip_credit: 0.0,
        };
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["giftCredit"], 100.0);
        assert_eq!(json["purchaseCredit"], 50.5);
        assert_eq!(json["vipCredit"], 0.0);
    }

    // -----------------------------------------------------------------------
    // parse_history_id
    // -----------------------------------------------------------------------

    #[test]
    fn parse_history_id_from_string() {
        let resp = json!({
            "data": { "aigc_data": { "history_record_id": "12977452690444" } }
        });
        assert_eq!(parse_history_id(&resp), "12977452690444");
    }

    #[test]
    fn parse_history_id_from_number() {
        let resp = json!({
            "data": { "aigc_data": { "history_record_id": 12977452690444u64 } }
        });
        assert_eq!(parse_history_id(&resp), "12977452690444");
    }

    #[test]
    fn parse_history_id_missing_field() {
        let resp = json!({ "data": { "aigc_data": {} } });
        assert_eq!(parse_history_id(&resp), "");
    }

    #[test]
    fn parse_history_id_missing_aigc_data() {
        let resp = json!({ "data": {} });
        assert_eq!(parse_history_id(&resp), "");
    }

    #[test]
    fn parse_history_id_empty_response() {
        let resp = json!({});
        assert_eq!(parse_history_id(&resp), "");
    }

    #[test]
    fn parse_history_id_null_value() {
        let resp = json!({
            "data": { "aigc_data": { "history_record_id": null } }
        });
        assert_eq!(parse_history_id(&resp), "");
    }

    // -----------------------------------------------------------------------
    // parse_credit_response
    // -----------------------------------------------------------------------

    #[test]
    fn parse_credit_full_response() {
        let resp = json!({
            "data": {
                "credit": {
                    "gift_credit": 80.0,
                    "purchase_credit": 0.0,
                    "vip_credit": 2154.0
                }
            }
        });
        let credit = parse_credit_response(&resp).unwrap();
        assert_eq!(credit.gift_credit, 80.0);
        assert_eq!(credit.purchase_credit, 0.0);
        assert_eq!(credit.vip_credit, 2154.0);
    }

    #[test]
    fn parse_credit_integer_values() {
        let resp = json!({
            "data": {
                "credit": {
                    "gift_credit": 100,
                    "purchase_credit": 50,
                    "vip_credit": 0
                }
            }
        });
        let credit = parse_credit_response(&resp).unwrap();
        assert_eq!(credit.gift_credit, 100.0);
        assert_eq!(credit.purchase_credit, 50.0);
        assert_eq!(credit.vip_credit, 0.0);
    }

    #[test]
    fn parse_credit_missing_fields_default_to_zero() {
        let resp = json!({
            "data": { "credit": { "gift_credit": 42.0 } }
        });
        let credit = parse_credit_response(&resp).unwrap();
        assert_eq!(credit.gift_credit, 42.0);
        assert_eq!(credit.purchase_credit, 0.0);
        assert_eq!(credit.vip_credit, 0.0);
    }

    #[test]
    fn parse_credit_missing_credit_key() {
        let resp = json!({ "data": {} });
        assert!(parse_credit_response(&resp).is_err());
    }

    #[test]
    fn parse_credit_empty_response() {
        let resp = json!({});
        assert!(parse_credit_response(&resp).is_err());
    }

    // -----------------------------------------------------------------------
    // TaskStatusResult / TaskItem deserialization from mock API data
    // -----------------------------------------------------------------------

    #[test]
    fn task_status_result_from_completed_response() {
        let data = json!({
            "status": 50,
            "failCode": "0",
            "itemList": [
                { "url": "https://example.com/img1.webp", "width": 2048, "height": 2048 },
                { "url": "https://example.com/img2.webp", "width": 2048, "height": 2048 }
            ]
        });
        let result: TaskStatusResult = serde_json::from_value(data).unwrap();
        assert_eq!(result.status, 50);
        assert_eq!(result.fail_code, "0");
        assert_eq!(result.item_list.len(), 2);
        assert_eq!(result.item_list[0].url, "https://example.com/img1.webp");
        assert_eq!(result.item_list[0].width, 2048);
    }

    #[test]
    fn task_status_result_queued_empty_items() {
        let data = json!({
            "status": 20,
            "failCode": "",
            "itemList": []
        });
        let result: TaskStatusResult = serde_json::from_value(data).unwrap();
        assert_eq!(result.status, 20);
        assert!(result.item_list.is_empty());
    }

    #[test]
    fn task_item_defaults_for_missing_fields() {
        let data = json!({});
        let item: TaskItem = serde_json::from_value(data).unwrap();
        assert_eq!(item.url, "");
        assert_eq!(item.width, 0);
        assert_eq!(item.height, 0);
    }

    // -----------------------------------------------------------------------
    // draft seed range
    // -----------------------------------------------------------------------

    #[test]
    fn draft_auto_seed_in_expected_range() {
        for _ in 0..20 {
            let draft = build_txt2img_draft("test", "m", "1:1", "", None, 0.5);
            let v: Value = serde_json::from_str(&draft).unwrap();
            let seed = v["component_list"][0]["abilities"]["generate"]["core_param"]["seed"]
                .as_u64()
                .unwrap();
            assert!(
                (2_500_000_000..2_600_000_000).contains(&seed),
                "seed {} should be in [2.5B, 2.6B)",
                seed
            );
        }
    }

    #[test]
    fn draft_explicit_seed_used() {
        let draft = build_txt2img_draft("test", "m", "1:1", "", Some(999), 0.5);
        let v: Value = serde_json::from_str(&draft).unwrap();
        let seed = v["component_list"][0]["abilities"]["generate"]["core_param"]["seed"]
            .as_u64()
            .unwrap();
        assert_eq!(seed, 999);
    }
}
