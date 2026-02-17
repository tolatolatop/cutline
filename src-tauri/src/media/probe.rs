use serde_json::Value;
use std::path::Path;
use std::process::Command;

pub fn ffprobe(file_path: &Path) -> Result<Value, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(file_path)
        .output()
        .map_err(|e| {
            format!(
                "执行 ffprobe 失败 (请确保已安装 FFmpeg): {}",
                e
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe 返回错误: {}", stderr));
    }

    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("解析 ffprobe 输出失败: {}", e))?;

    Ok(json)
}

pub fn extract_video_meta(probe_data: &Value) -> Value {
    let streams = probe_data
        .get("streams")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let format = probe_data.get("format").cloned().unwrap_or(Value::Null);

    let video_stream = streams.iter().find(|s| {
        s.get("codec_type")
            .and_then(|v| v.as_str())
            .map(|v| v == "video")
            .unwrap_or(false)
    });

    let audio_stream = streams.iter().find(|s| {
        s.get("codec_type")
            .and_then(|v| v.as_str())
            .map(|v| v == "audio")
            .unwrap_or(false)
    });

    let duration_sec = format
        .get("duration")
        .and_then(|d| d.as_str())
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    let container = format
        .get("format_name")
        .and_then(|f| f.as_str())
        .unwrap_or("unknown")
        .to_string();

    if let Some(vs) = video_stream {
        let codec = vs
            .get("codec_name")
            .and_then(|c| c.as_str())
            .unwrap_or("unknown");
        let width = vs
            .get("width")
            .and_then(|w| w.as_u64())
            .unwrap_or(0) as u32;
        let height = vs
            .get("height")
            .and_then(|h| h.as_u64())
            .unwrap_or(0) as u32;

        let fps = parse_fps(
            vs.get("r_frame_rate")
                .and_then(|f| f.as_str())
                .unwrap_or("0/1"),
        );

        let audio_meta = audio_stream.map(|a| {
            let sample_rate = a
                .get("sample_rate")
                .and_then(|s| s.as_str())
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            let channels = a
                .get("channels")
                .and_then(|c| c.as_u64())
                .unwrap_or(0) as u32;
            serde_json::json!({
                "present": true,
                "sampleRate": sample_rate,
                "channels": channels
            })
        });

        serde_json::json!({
            "kind": "video",
            "container": container,
            "codec": codec,
            "durationSec": duration_sec,
            "width": width,
            "height": height,
            "fps": fps,
            "audio": audio_meta.unwrap_or(serde_json::json!(null))
        })
    } else if let Some(a) = audio_stream {
        let codec = a
            .get("codec_name")
            .and_then(|c| c.as_str())
            .unwrap_or("unknown");
        let sample_rate = a
            .get("sample_rate")
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let channels = a
            .get("channels")
            .and_then(|c| c.as_u64())
            .unwrap_or(0) as u32;

        serde_json::json!({
            "kind": "audio",
            "codec": codec,
            "durationSec": duration_sec,
            "sampleRate": sample_rate,
            "channels": channels
        })
    } else {
        serde_json::json!({
            "kind": "unknown"
        })
    }
}

pub fn extract_image_meta(file_path: &Path) -> Value {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown")
        .to_lowercase();

    if let Ok(probe_data) = ffprobe(file_path) {
        let streams = probe_data
            .get("streams")
            .and_then(|s| s.as_array())
            .cloned()
            .unwrap_or_default();

        if let Some(vs) = streams.first() {
            let width = vs
                .get("width")
                .and_then(|w| w.as_u64())
                .unwrap_or(0) as u32;
            let height = vs
                .get("height")
                .and_then(|h| h.as_u64())
                .unwrap_or(0) as u32;

            return serde_json::json!({
                "kind": "image",
                "format": ext,
                "width": width,
                "height": height
            });
        }
    }

    serde_json::json!({
        "kind": "image",
        "format": ext,
        "width": 0,
        "height": 0
    })
}

fn parse_fps(rate: &str) -> f64 {
    let parts: Vec<&str> = rate.split('/').collect();
    if parts.len() == 2 {
        let num: f64 = parts[0].parse().unwrap_or(0.0);
        let den: f64 = parts[1].parse().unwrap_or(1.0);
        if den > 0.0 {
            return (num / den * 100.0).round() / 100.0;
        }
    }
    0.0
}
