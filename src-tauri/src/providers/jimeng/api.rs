use std::collections::HashMap;

use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::client::JimengClient;
use super::constants::{
    get_aspect_ratio, resolve_model, APP_ID, AspectRatio, DRAFT_VERSION,
    SEEDANCE_DEFAULT_FPS, SEEDANCE_DEFAULT_DURATION_MS,
    SEEDANCE_VIDEO_MODE,
    VIDEO_DRAFT_VERSION, VIDEO_MIN_VERSION, VIDEO_BENEFIT_TYPE, SEEDANCE_BENEFIT_TYPE,
};

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResult {
    pub history_id: String,
    pub submit_id: String,
}

/// Per-history task status, deserialized from API (snake_case) and serialized to frontend (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusResult {
    #[serde(default)]
    pub status: u32,
    #[serde(alias = "fail_code", default)]
    pub fail_code: String,
    #[serde(alias = "fail_msg", default)]
    pub fail_msg: String,
    #[serde(alias = "item_list", default)]
    pub item_list: Vec<TaskItem>,
    #[serde(alias = "history_record_id", default)]
    pub history_record_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub video: Option<VideoInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoInfo {
    #[serde(default, alias = "video_url")]
    pub video_url: String,
    #[serde(default, alias = "transcoded_video")]
    pub transcoded_video: Option<TranscodedVideo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodedVideo {
    #[serde(default)]
    pub origin: Option<VideoOrigin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoOrigin {
    #[serde(default, alias = "video_url")]
    pub video_url: String,
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
    aspect: &AspectRatio,
    negative_prompt: &str,
    seed: Option<u64>,
    sample_strength: f64,
) -> String {
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
            "gen_type": 1,
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
// video draft_content builder (gen_video.text_to_video_params format)
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub(crate) fn build_text2video_draft(
    prompt: &str,
    model: &str,
    ratio: &str,
    duration_ms: Option<u32>,
) -> String {
    let duration_ms = duration_ms.unwrap_or(SEEDANCE_DEFAULT_DURATION_MS);
    let component_id = new_uuid();

    let metrics_extra = json!({
        "enterFrom": "click",
        "isDefaultSeed": 1,
        "promptSource": "custom",
        "isRegenerate": false,
        "originSubmitId": new_uuid(),
    });

    let draft = json!({
        "type": "draft",
        "id": new_uuid(),
        "min_version": VIDEO_MIN_VERSION,
        "is_from_tsn": true,
        "version": VIDEO_DRAFT_VERSION,
        "main_component_id": component_id,
        "component_list": [{
            "type": "video_base_component",
            "id": component_id,
            "min_version": "1.0.0",
            "metadata": {
                "type": "",
                "id": new_uuid(),
                "created_platform": 3,
                "created_platform_version": "",
                "created_time_in_ms": now_ms(),
                "created_did": ""
            },
            "generate_type": "gen_video",
            "aigc_mode": "workbench",
            "abilities": {
                "type": "",
                "id": new_uuid(),
                "gen_video": {
                    "id": new_uuid(),
                    "type": "",
                    "text_to_video_params": {
                        "type": "",
                        "id": new_uuid(),
                        "model_req_key": model,
                        "priority": 0,
                        "seed": random_seed(),
                        "video_aspect_ratio": ratio,
                        "video_gen_inputs": [{
                            "duration_ms": duration_ms,
                            "fps": SEEDANCE_DEFAULT_FPS,
                            "id": new_uuid(),
                            "min_version": VIDEO_MIN_VERSION,
                            "prompt": prompt,
                            "resolution": "720p",
                            "type": "",
                            "video_mode": 2
                        }]
                    },
                    "video_task_extra": metrics_extra.to_string(),
                }
            }
        }]
    });

    draft.to_string()
}

pub(crate) fn build_video_metrics_extra() -> String {
    json!({
        "enterFrom": "click",
        "isDefaultSeed": 1,
        "promptSource": "custom",
        "isRegenerate": false,
        "originSubmitId": new_uuid(),
    })
    .to_string()
}

pub(crate) fn build_seedance_draft(
    prompt: &str,
    internal_model: &str,
    ratio: &str,
    duration_ms: Option<u32>,
    video_task_extra: &str,
) -> String {
    let dur = duration_ms.unwrap_or(SEEDANCE_DEFAULT_DURATION_MS);
    let seed: u64 = rand::thread_rng().gen_range(1_000_000_000..2_600_000_000);

    let component_id = new_uuid();

    let draft = json!({
        "type": "draft",
        "id": new_uuid(),
        "min_version": VIDEO_MIN_VERSION,
        "min_features": [],
        "is_from_tsn": true,
        "version": VIDEO_DRAFT_VERSION,
        "main_component_id": component_id,
        "component_list": [{
            "type": "video_base_component",
            "id": component_id,
            "min_version": "1.0.0",
            "aigc_mode": "workbench",
            "metadata": {
                "type": "",
                "id": new_uuid(),
                "created_platform": 3,
                "created_platform_version": "",
                "created_time_in_ms": now_ms().to_string(),
                "created_did": ""
            },
            "generate_type": "gen_video",
            "abilities": {
                "type": "",
                "id": new_uuid(),
                "gen_video": {
                    "type": "",
                    "id": new_uuid(),
                    "text_to_video_params": {
                        "type": "",
                        "id": new_uuid(),
                        "video_gen_inputs": [{
                            "type": "",
                            "id": new_uuid(),
                            "min_version": VIDEO_MIN_VERSION,
                            "prompt": prompt,
                            "video_mode": SEEDANCE_VIDEO_MODE,
                            "fps": SEEDANCE_DEFAULT_FPS,
                            "duration_ms": dur,
                            "idip_meta_list": []
                        }],
                        "video_aspect_ratio": ratio,
                        "seed": seed,
                        "model_req_key": internal_model,
                        "priority": 0
                    },
                    "video_task_extra": video_task_extra
                }
            },
            "process_type": 1
        }]
    });

    draft.to_string()
}

pub(crate) fn build_seedance_metrics_extra(internal_model: &str, duration_ms: u32, submit_id: &str) -> String {
    let scene_options = json!([{
        "type": "video",
        "scene": "BasicVideoGenerateButton",
        "modelReqKey": internal_model,
        "videoDuration": duration_ms / 1000,
        "reportParams": {
            "enterSource": "generate",
            "vipSource": "generate",
            "extraVipFunctionKey": internal_model,
            "useVipFunctionDetailsReporterHoc": true
        },
        "materialTypes": []
    }]);

    json!({
        "promptSource": "custom",
        "isDefaultSeed": 1,
        "originSubmitId": submit_id,
        "isRegenerate": false,
        "enterFrom": "click",
        "position": "page_bottom_box",
        "functionMode": "first_last_frames",
        "sceneOptions": scene_options.to_string()
    })
    .to_string()
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

fn parse_submit_id(resp: &Value) -> String {
    resp.pointer("/data/aigc_data/task/submit_id")
        .or_else(|| resp.pointer("/data/aigc_data/submit_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn extract_video_url(task_result: &TaskStatusResult) -> Option<String> {
    for item in &task_result.item_list {
        if let Some(video) = &item.video {
            if let Some(transcoded) = &video.transcoded_video {
                if let Some(origin) = &transcoded.origin {
                    if !origin.video_url.is_empty() {
                        return Some(origin.video_url.clone());
                    }
                }
            }
            if !video.video_url.is_empty() {
                return Some(video.video_url.clone());
            }
        }
        if !item.url.is_empty() {
            return Some(item.url.clone());
        }
    }
    None
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
        &aspect,
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
        "http_common_info": { "aid": APP_ID.parse::<u64>().unwrap() }
    });

    let resp = client.post(GENERATE_PATH, &body, &internal_model, false, None).await?;
    let history_id = parse_history_id(&resp);

    Ok(GenerateResult {
        history_id,
        submit_id,
    })
}

pub async fn generate_video(
    client: &JimengClient,
    prompt: &str,
    model: &str,
    ratio: &str,
    duration_ms: Option<u32>,
) -> Result<GenerateResult, String> {
    let internal_model = resolve_model(model);
    let is_seedance = internal_model.contains("seedance");

    let submit_id = new_uuid();

    let (draft, metrics_extra, benefit_type) = if is_seedance {
        let dur = duration_ms.unwrap_or(SEEDANCE_DEFAULT_DURATION_MS);
        let metrics = build_seedance_metrics_extra(&internal_model, dur, &submit_id);
        let draft = build_seedance_draft(prompt, &internal_model, ratio, duration_ms, &metrics);
        (draft, metrics, SEEDANCE_BENEFIT_TYPE)
    } else {
        let draft = build_text2video_draft(prompt, &internal_model, ratio, duration_ms);
        let metrics = build_video_metrics_extra();
        (draft, metrics, VIDEO_BENEFIT_TYPE)
    };

    log::info!("[generate_video] internal_model={}, benefit_type={}, seedance={}", internal_model, benefit_type, is_seedance);
    log::info!("[generate_video] draft_content={}", draft);

    let body = json!({
        "extend": {
            "root_model": internal_model,
            "m_video_commerce_info": {
                "benefit_type": benefit_type,
                "resource_id": "generate_video",
                "resource_id_type": "str",
                "resource_sub_type": "aigc"
            },
            "m_video_commerce_info_list": [{
                "benefit_type": benefit_type,
                "resource_id": "generate_video",
                "resource_id_type": "str",
                "resource_sub_type": "aigc"
            }]
        },
        "submit_id": submit_id,
        "metrics_extra": metrics_extra,
        "draft_content": draft,
        "http_common_info": { "aid": APP_ID.parse::<u64>().unwrap() }
    });

    let resp = client.post(GENERATE_PATH, &body, &internal_model, false, None).await?;

    log::info!("[generate_video] full response: {}", serde_json::to_string_pretty(&resp).unwrap_or_default());

    let history_id = parse_history_id(&resp);
    let server_submit_id = parse_submit_id(&resp);

    log::info!("[generate_video] parsed: history_id={}, submit_id={}", history_id, server_submit_id);

    Ok(GenerateResult {
        history_id,
        submit_id: if server_submit_id.is_empty() { submit_id } else { server_submit_id },
    })
}

// ---------------------------------------------------------------------------
// Task status
// ---------------------------------------------------------------------------

fn parse_task_status(resp: &Value, history_ids: &[String]) -> Result<HashMap<String, TaskStatusResult>, String> {
    let data = resp.get("data").ok_or("Missing 'data' in task status response")?;
    let mut results = HashMap::new();

    for hid in history_ids {
        if let Some(entry) = data.get(hid) {
            let status: TaskStatusResult = serde_json::from_value(entry.clone())
                .map_err(|e| format!("Failed to parse task status for {}: {}", hid, e))?;
            results.insert(hid.clone(), status);
        }
    }

    Ok(results)
}

pub async fn get_task_status(
    client: &JimengClient,
    history_ids: &[String],
    submit_ids: Option<&[String]>,
) -> Result<HashMap<String, TaskStatusResult>, String> {
    let mut body = json!({
        "history_ids": history_ids,
        "image_info": {
            "width": 2048,
            "height": 2048,
            "format": "webp",
            "image_scene_list": []
        },
        "http_common_info": { "aid": APP_ID.parse::<u64>().unwrap() }
    });

    if let Some(sids) = submit_ids {
        body["submit_ids"] = json!(sids);
    }

    let resp = client.post(HISTORY_PATH, &body, "", false, None).await?;

    let lookup_ids: Vec<String> = if let Some(sids) = submit_ids {
        history_ids.iter().chain(sids.iter()).cloned().collect()
    } else {
        history_ids.to_vec()
    };
    parse_task_status(&resp, &lookup_ids)
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
        let draft = build_txt2img_draft("test prompt", "high_aes_general_v40l", &get_aspect_ratio("1:1"), "", None, 0.5);
        let parsed: Value = serde_json::from_str(&draft).expect("draft should be valid JSON");
        assert_eq!(parsed["type"], "draft");
    }

    #[test]
    fn draft_has_required_top_level_fields() {
        let draft = build_txt2img_draft("hello", "model_v1", &get_aspect_ratio("16:9"), "", None, 0.5);
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
        let draft = build_txt2img_draft("cat", "model_v1", &get_aspect_ratio("1:1"), "ugly", Some(12345), 0.7);
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
        let draft = build_txt2img_draft("test", "m", &get_aspect_ratio("16:9"), "", None, 0.5);
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
        let draft = build_txt2img_draft("test", "m", &get_aspect_ratio("1:1"), "", None, 0.5);
        let v: Value = serde_json::from_str(&draft).unwrap();

        let main_id = v["main_component_id"].as_str().unwrap();
        let comp_id = v["component_list"][0]["id"].as_str().unwrap();
        assert_eq!(main_id, comp_id);
    }

    #[test]
    fn draft_uuids_are_unique() {
        let draft = build_txt2img_draft("test", "m", &get_aspect_ratio("1:1"), "", None, 0.5);
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
    fn task_status_result_from_api_snake_case() {
        let data = json!({
            "status": 50,
            "fail_code": "0",
            "history_record_id": "12984053829900",
            "item_list": [
                { "url": "https://example.com/img1.webp", "width": 2048, "height": 2048 },
                { "url": "https://example.com/img2.webp", "width": 2048, "height": 2048 }
            ]
        });
        let result: TaskStatusResult = serde_json::from_value(data).unwrap();
        assert_eq!(result.status, 50);
        assert_eq!(result.fail_code, "0");
        assert_eq!(result.history_record_id, "12984053829900");
        assert_eq!(result.item_list.len(), 2);
        assert_eq!(result.item_list[0].url, "https://example.com/img1.webp");
        assert_eq!(result.item_list[0].width, 2048);
    }

    #[test]
    fn task_status_result_queued_empty_items() {
        let data = json!({
            "status": 20,
            "fail_code": "",
            "item_list": []
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

    #[test]
    fn parse_task_status_multiple_ids() {
        let resp = json!({
            "data": {
                "111": { "status": 50, "fail_code": "0", "item_list": [{ "url": "a.webp" }], "history_record_id": "111" },
                "222": { "status": 20, "fail_code": "", "item_list": [], "history_record_id": "222" }
            }
        });
        let ids = vec!["111".to_string(), "222".to_string()];
        let result = parse_task_status(&resp, &ids).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result["111"].status, 50);
        assert_eq!(result["111"].item_list.len(), 1);
        assert_eq!(result["222"].status, 20);
    }

    #[test]
    fn parse_task_status_missing_id_skipped() {
        let resp = json!({ "data": {} });
        let ids = vec!["999".to_string()];
        let result = parse_task_status(&resp, &ids).unwrap();
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // draft seed range
    // -----------------------------------------------------------------------

    #[test]
    fn draft_auto_seed_in_expected_range() {
        for _ in 0..20 {
            let draft = build_txt2img_draft("test", "m", &get_aspect_ratio("1:1"), "", None, 0.5);
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
        let draft = build_txt2img_draft("test", "m", &get_aspect_ratio("1:1"), "", Some(999), 0.5);
        let v: Value = serde_json::from_str(&draft).unwrap();
        let seed = v["component_list"][0]["abilities"]["generate"]["core_param"]["seed"]
            .as_u64()
            .unwrap();
        assert_eq!(seed, 999);
    }

    // -----------------------------------------------------------------------
    // build_text2video_draft (gen_video.text_to_video_params format)
    // -----------------------------------------------------------------------

    #[test]
    fn video_draft_is_valid_json() {
        let draft = build_text2video_draft("test video", "model_v1", "16:9", None);
        let v: Value = serde_json::from_str(&draft).expect("video draft should be valid JSON");
        assert_eq!(v["type"], "draft");
        assert_eq!(v["version"], VIDEO_DRAFT_VERSION);
    }

    #[test]
    fn video_draft_structure() {
        let draft = build_text2video_draft("a cat running", "model_v1", "16:9", Some(8000));
        let v: Value = serde_json::from_str(&draft).unwrap();

        assert_eq!(v["type"], "draft");
        assert!(v["id"].is_string());
        assert_eq!(v["version"], VIDEO_DRAFT_VERSION);
        assert_eq!(v["min_version"], VIDEO_MIN_VERSION);
        assert!(v["is_from_tsn"].as_bool().unwrap());
        assert!(v["main_component_id"].is_string());

        let comp = &v["component_list"][0];
        assert_eq!(comp["type"], "video_base_component");
        assert!(comp["id"].is_string());
        assert_eq!(comp["generate_type"], "gen_video");
        assert_eq!(comp["aigc_mode"], "workbench");

        let t2v = &comp["abilities"]["gen_video"]["text_to_video_params"];
        assert_eq!(t2v["model_req_key"], "model_v1");
        assert_eq!(t2v["video_aspect_ratio"], "16:9");
        assert_eq!(t2v["priority"], 0);
        assert!(t2v["seed"].is_u64());

        let input = &t2v["video_gen_inputs"][0];
        assert_eq!(input["prompt"], "a cat running");
        assert_eq!(input["duration_ms"], 8000);
        assert_eq!(input["fps"], SEEDANCE_DEFAULT_FPS);
        assert_eq!(input["resolution"], "720p");
        assert_eq!(input["video_mode"], 2);
    }

    #[test]
    fn video_draft_default_duration() {
        let draft = build_text2video_draft("test", "m", "1:1", None);
        let v: Value = serde_json::from_str(&draft).unwrap();
        let dur = v["component_list"][0]["abilities"]["gen_video"]["text_to_video_params"]["video_gen_inputs"][0]["duration_ms"]
            .as_u64().unwrap();
        assert_eq!(dur, SEEDANCE_DEFAULT_DURATION_MS as u64);
    }

    #[test]
    fn video_draft_ratio_passed_through() {
        for ratio in &["16:9", "9:16", "1:1"] {
            let draft = build_text2video_draft("test", "m", ratio, None);
            let v: Value = serde_json::from_str(&draft).unwrap();
            assert_eq!(
                v["component_list"][0]["abilities"]["gen_video"]["text_to_video_params"]["video_aspect_ratio"].as_str().unwrap(),
                *ratio,
            );
        }
    }

    #[test]
    fn video_draft_main_component_id_matches() {
        let draft = build_text2video_draft("test", "m", "16:9", None);
        let v: Value = serde_json::from_str(&draft).unwrap();
        let main_id = v["main_component_id"].as_str().unwrap();
        let comp_id = v["component_list"][0]["id"].as_str().unwrap();
        assert_eq!(main_id, comp_id);
    }

    // -----------------------------------------------------------------------
    // parse_submit_id
    // -----------------------------------------------------------------------

    #[test]
    fn parse_submit_id_from_task() {
        let resp = json!({
            "data": { "aigc_data": { "task": { "submit_id": "abc-123" } } }
        });
        assert_eq!(parse_submit_id(&resp), "abc-123");
    }

    #[test]
    fn parse_submit_id_from_aigc_data() {
        let resp = json!({
            "data": { "aigc_data": { "submit_id": "xyz-456" } }
        });
        assert_eq!(parse_submit_id(&resp), "xyz-456");
    }

    #[test]
    fn parse_submit_id_missing() {
        let resp = json!({ "data": { "aigc_data": {} } });
        assert_eq!(parse_submit_id(&resp), "");
    }

    // -----------------------------------------------------------------------
    // extract_video_url
    // -----------------------------------------------------------------------

    #[test]
    fn extract_video_url_from_transcoded() {
        let result = TaskStatusResult {
            status: 50,
            fail_code: "0".into(),
            fail_msg: String::new(),
            history_record_id: "123".into(),
            item_list: vec![TaskItem {
                url: "".into(),
                width: 1280,
                height: 720,
                video: Some(VideoInfo {
                    video_url: "fallback.mp4".into(),
                    transcoded_video: Some(TranscodedVideo {
                        origin: Some(VideoOrigin {
                            video_url: "https://example.com/transcoded.mp4".into(),
                        }),
                    }),
                }),
            }],
        };
        assert_eq!(
            extract_video_url(&result),
            Some("https://example.com/transcoded.mp4".into())
        );
    }

    #[test]
    fn extract_video_url_fallback_to_video_url() {
        let result = TaskStatusResult {
            status: 50,
            fail_code: "0".into(),
            fail_msg: String::new(),
            history_record_id: "123".into(),
            item_list: vec![TaskItem {
                url: "".into(),
                width: 0,
                height: 0,
                video: Some(VideoInfo {
                    video_url: "https://example.com/direct.mp4".into(),
                    transcoded_video: None,
                }),
            }],
        };
        assert_eq!(
            extract_video_url(&result),
            Some("https://example.com/direct.mp4".into())
        );
    }

    #[test]
    fn extract_video_url_fallback_to_item_url() {
        let result = TaskStatusResult {
            status: 50,
            fail_code: "0".into(),
            fail_msg: String::new(),
            history_record_id: "123".into(),
            item_list: vec![TaskItem {
                url: "https://example.com/item.mp4".into(),
                width: 0,
                height: 0,
                video: None,
            }],
        };
        assert_eq!(
            extract_video_url(&result),
            Some("https://example.com/item.mp4".into())
        );
    }

    #[test]
    fn extract_video_url_empty_items() {
        let result = TaskStatusResult {
            status: 50,
            fail_code: "0".into(),
            fail_msg: String::new(),
            history_record_id: "123".into(),
            item_list: vec![],
        };
        assert_eq!(extract_video_url(&result), None);
    }
}
