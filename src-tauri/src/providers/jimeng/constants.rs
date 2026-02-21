use std::collections::HashMap;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// 应用级常量
// ---------------------------------------------------------------------------
pub const APP_ID: &str = "513695";
pub const APP_VERSION: &str = "8.4.0";
pub const WEB_VERSION: &str = "7.5.0";
pub const DA_VERSION: &str = "3.3.9";
pub const AIGC_FEATURES: &str = "app_lip_sync";
pub const APP_SDK_VERSION: &str = "48.0.0";
pub const PLATFORM_CODE: &str = "7";
pub const SIGN_PREFIX: &str = "9e2c";
pub const SIGN_SUFFIX: &str = "11ac";
pub const BASE_URL: &str = "https://jimeng.jianying.com";

pub const DRAFT_VERSION: &str = "3.0.2";
pub const VIDEO_DRAFT_VERSION: &str = "3.3.2";
pub const VIDEO_MIN_VERSION: &str = "3.0.5";
pub const VIDEO_BENEFIT_TYPE: &str = "basic_video_operation_vgfm_v_three";
pub const SEEDANCE_BENEFIT_TYPE: &str = "dreamina_seedance_20_fast";
pub const SEEDANCE_VERSION: &str = "3.3.9";
pub const SEEDANCE_MIN_FEATURE: &str = "AIGC_Video_UnifiedEdit";
pub const SEEDANCE_VIDEO_MODE: u32 = 2;

pub const SEEDANCE_DEFAULT_FPS: u32 = 24;
pub const SEEDANCE_DEFAULT_DURATION_MS: u32 = 5000;

// ---------------------------------------------------------------------------
// 图片模型映射: 用户友好名 -> 内部 req_key
// ---------------------------------------------------------------------------
pub static IMAGE_MODELS: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    HashMap::from([
        ("jimeng-4.5", "high_aes_general_v40l"),
        ("jimeng-4.1", "high_aes_general_v41"),
        ("jimeng-4.0", "high_aes_general_v40"),
        (
            "jimeng-3.1",
            "high_aes_general_v30l_art_fangzhou:general_v3.0_18b",
        ),
        ("jimeng-3.0", "high_aes_general_v30l:general_v3.0_18b"),
        ("jimeng-2.1", "high_aes_general_v21_L:general_v2.1_L"),
        ("jimeng-2.0-pro", "high_aes_general_v20_L:general_v2.0_L"),
        ("jimeng-2.0", "high_aes_general_v20:general_v2.0"),
        ("jimeng-1.4", "high_aes_general_v14:general_v1.4"),
        ("jimeng-xl-pro", "text2img_xl_sft"),
    ])
});

// ---------------------------------------------------------------------------
// 视频模型映射
// ---------------------------------------------------------------------------
pub static VIDEO_MODELS: LazyLock<HashMap<&str, &str>> = LazyLock::new(|| {
    HashMap::from([
        (
            "jimeng-video-3.0",
            "dreamina_ic_generate_video_model_vgfm_3.0",
        ),
        (
            "jimeng-video-3.0-pro",
            "dreamina_ic_generate_video_model_vgfm_3.0_pro",
        ),
        (
            "jimeng-video-2.0-pro",
            "dreamina_ic_generate_video_model_vgfm1.0",
        ),
        (
            "jimeng-video-2.0",
            "dreamina_ic_generate_video_model_vgfm_lite",
        ),
        ("seedance-2.0", "dreamina_seedance_40"),
    ])
});

/// 将用户模型名解析为内部名称，找不到则原样返回。
pub fn resolve_model(name: &str) -> String {
    if let Some(v) = IMAGE_MODELS.get(name) {
        return v.to_string();
    }
    if let Some(v) = VIDEO_MODELS.get(name) {
        return v.to_string();
    }
    name.to_string()
}

// ---------------------------------------------------------------------------
// 宽高比预设
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub struct AspectSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct AspectRatio {
    pub ratio_type: u32,
    pub size_2k: AspectSize,
}

pub static ASPECT_RATIOS: LazyLock<HashMap<&str, AspectRatio>> = LazyLock::new(|| {
    HashMap::from([
        (
            "1:1",
            AspectRatio {
                ratio_type: 1,
                size_2k: AspectSize {
                    width: 2048,
                    height: 2048,
                },
            },
        ),
        (
            "3:4",
            AspectRatio {
                ratio_type: 2,
                size_2k: AspectSize {
                    width: 1728,
                    height: 2304,
                },
            },
        ),
        (
            "16:9",
            AspectRatio {
                ratio_type: 3,
                size_2k: AspectSize {
                    width: 2560,
                    height: 1440,
                },
            },
        ),
        (
            "4:3",
            AspectRatio {
                ratio_type: 4,
                size_2k: AspectSize {
                    width: 2304,
                    height: 1728,
                },
            },
        ),
        (
            "9:16",
            AspectRatio {
                ratio_type: 5,
                size_2k: AspectSize {
                    width: 1440,
                    height: 2560,
                },
            },
        ),
        (
            "2:3",
            AspectRatio {
                ratio_type: 6,
                size_2k: AspectSize {
                    width: 1664,
                    height: 2496,
                },
            },
        ),
        (
            "3:2",
            AspectRatio {
                ratio_type: 7,
                size_2k: AspectSize {
                    width: 2496,
                    height: 1664,
                },
            },
        ),
        (
            "21:9",
            AspectRatio {
                ratio_type: 8,
                size_2k: AspectSize {
                    width: 3024,
                    height: 1296,
                },
            },
        ),
    ])
});

pub fn get_aspect_ratio(name: &str) -> AspectRatio {
    ASPECT_RATIOS
        .get(name)
        .copied()
        .unwrap_or_else(|| *ASPECT_RATIOS.get("1:1").unwrap())
}

// ---------------------------------------------------------------------------
// 任务状态码
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TaskStatus {
    Queued = 20,
    Failed = 30,
    Partial = 42,
    Processing = 45,
    Completed = 50,
}

impl TaskStatus {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            20 => Some(Self::Queued),
            30 => Some(Self::Failed),
            42 => Some(Self::Partial),
            45 => Some(Self::Processing),
            50 => Some(Self::Completed),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_image_model() {
        assert_eq!(resolve_model("jimeng-4.5"), "high_aes_general_v40l");
        assert_eq!(resolve_model("jimeng-4.0"), "high_aes_general_v40");
        assert_eq!(resolve_model("jimeng-xl-pro"), "text2img_xl_sft");
    }

    #[test]
    fn resolve_known_video_model() {
        assert_eq!(
            resolve_model("seedance-2.0"),
            "dreamina_seedance_40"
        );
        assert_eq!(
            resolve_model("jimeng-video-3.0"),
            "dreamina_ic_generate_video_model_vgfm_3.0"
        );
    }

    #[test]
    fn resolve_unknown_model_returns_as_is() {
        assert_eq!(resolve_model("unknown-model"), "unknown-model");
        assert_eq!(resolve_model("custom_v1"), "custom_v1");
    }

    #[test]
    fn all_image_models_resolvable() {
        let names = [
            "jimeng-4.5", "jimeng-4.1", "jimeng-4.0", "jimeng-3.1", "jimeng-3.0",
            "jimeng-2.1", "jimeng-2.0-pro", "jimeng-2.0", "jimeng-1.4", "jimeng-xl-pro",
        ];
        for name in names {
            let resolved = resolve_model(name);
            assert_ne!(resolved, name, "{} should resolve to internal name", name);
        }
    }

    #[test]
    fn aspect_ratio_known_ratios() {
        let r = get_aspect_ratio("1:1");
        assert_eq!(r.ratio_type, 1);
        assert_eq!(r.size_2k.width, 2048);
        assert_eq!(r.size_2k.height, 2048);

        let r = get_aspect_ratio("16:9");
        assert_eq!(r.ratio_type, 3);
        assert_eq!(r.size_2k.width, 2560);
        assert_eq!(r.size_2k.height, 1440);

        let r = get_aspect_ratio("9:16");
        assert_eq!(r.ratio_type, 5);
        assert_eq!(r.size_2k.width, 1440);
        assert_eq!(r.size_2k.height, 2560);
    }

    #[test]
    fn aspect_ratio_unknown_defaults_to_1_1() {
        let r = get_aspect_ratio("7:3");
        assert_eq!(r.ratio_type, 1);
        assert_eq!(r.size_2k.width, 2048);
    }

    #[test]
    fn all_8_aspect_ratios_exist() {
        let names = ["1:1", "3:4", "16:9", "4:3", "9:16", "2:3", "3:2", "21:9"];
        for name in names {
            assert!(
                ASPECT_RATIOS.get(name).is_some(),
                "Aspect ratio {} should exist",
                name
            );
        }
    }

    #[test]
    fn task_status_from_u32() {
        assert_eq!(TaskStatus::from_u32(20), Some(TaskStatus::Queued));
        assert_eq!(TaskStatus::from_u32(30), Some(TaskStatus::Failed));
        assert_eq!(TaskStatus::from_u32(42), Some(TaskStatus::Partial));
        assert_eq!(TaskStatus::from_u32(45), Some(TaskStatus::Processing));
        assert_eq!(TaskStatus::from_u32(50), Some(TaskStatus::Completed));
        assert_eq!(TaskStatus::from_u32(0), None);
        assert_eq!(TaskStatus::from_u32(99), None);
    }

    #[test]
    fn constants_values_match_python() {
        assert_eq!(APP_ID, "513695");
        assert_eq!(APP_VERSION, "8.4.0");
        assert_eq!(WEB_VERSION, "7.5.0");
        assert_eq!(DA_VERSION, "3.3.9");
        assert_eq!(PLATFORM_CODE, "7");
        assert_eq!(SIGN_PREFIX, "9e2c");
        assert_eq!(SIGN_SUFFIX, "11ac");
        assert_eq!(BASE_URL, "https://jimeng.jianying.com");
    }
}
