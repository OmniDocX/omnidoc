use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const DEFAULT_BASE: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
const DEFAULT_MODEL: &str = "qwen3.7-flash";
const DEFAULT_OCR_MODEL: &str = "qwen3.5-ocr";

fn env_value(primary: &str, compatible: &str) -> Option<String> {
    std::env::var(primary)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var(compatible)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
}

fn local_config_path() -> PathBuf {
    std::env::var_os("UNIPPT_AI_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".env.local"))
}

fn clean_image_config_path() -> PathBuf {
    std::env::var_os("UNIPPT_CLEAN_IMAGE_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".env.clean-image.local"))
}

fn local_values(path: &Path) -> BTreeMap<String, String> {
    let Ok(iter) = dotenvy::from_path_iter(path) else {
        return BTreeMap::new();
    };
    iter.filter_map(Result::ok).collect()
}

fn local_value(
    values: &BTreeMap<String, String>,
    primary: &str,
    compatible: &str,
) -> Option<String> {
    values
        .get(primary)
        .or_else(|| values.get(compatible))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

struct AiConfig {
    base: String,
    model: String,
    ocr_model: String,
    key: Option<String>,
    key_source: &'static str,
    base_configured: bool,
}

struct CleanImageConfig {
    url: Option<String>,
    key: Option<String>,
    key_source: &'static str,
}

fn clean_image_config() -> CleanImageConfig {
    let values = local_values(&clean_image_config_path());
    let env_url = std::env::var("UNIPPT_CLEAN_IMAGE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let env_key = std::env::var("UNIPPT_CLEAN_IMAGE_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let local_url = values
        .get("UNIPPT_CLEAN_IMAGE_URL")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let local_key = values
        .get("UNIPPT_CLEAN_IMAGE_KEY")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let key_source = if env_key.is_some() {
        "server-env"
    } else if local_key.is_some() {
        "local-config"
    } else {
        "missing"
    };
    CleanImageConfig {
        url: env_url.or(local_url),
        key: env_key.or(local_key),
        key_source,
    }
}

fn config() -> AiConfig {
    let values = local_values(&local_config_path());
    let env_base = env_value("UNIPPT_AI_BASE", "UNIDOC_AI_BASE");
    let env_model = env_value("UNIPPT_AI_MODEL", "UNIDOC_AI_MODEL");
    let env_ocr_model = env_value("UNIPPT_OCR_MODEL", "UNIDOC_OCR_MODEL");
    let env_key = env_value("UNIPPT_AI_KEY", "UNIDOC_AI_KEY");
    let local_base = local_value(&values, "UNIPPT_AI_BASE", "UNIDOC_AI_BASE");
    let local_model = local_value(&values, "UNIPPT_AI_MODEL", "UNIDOC_AI_MODEL");
    let local_ocr_model = local_value(&values, "UNIPPT_OCR_MODEL", "UNIDOC_OCR_MODEL");
    let local_key = local_value(&values, "UNIPPT_AI_KEY", "UNIDOC_AI_KEY");
    let key_source = if env_key.is_some() {
        "server-env"
    } else if local_key.is_some() {
        "local-config"
    } else {
        "missing"
    };
    AiConfig {
        base_configured: env_base.is_some() || local_base.is_some(),
        base: env_base
            .or(local_base)
            .unwrap_or_else(|| DEFAULT_BASE.to_string()),
        model: env_model
            .or(local_model)
            .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        ocr_model: env_ocr_model
            .or(local_ocr_model)
            .unwrap_or_else(|| DEFAULT_OCR_MODEL.to_string()),
        key: env_key.or(local_key),
        key_source,
    }
}

pub fn status() -> Value {
    let config = config();
    let clean_image = clean_image_config();
    let provider = if config.base.contains("dashscope.aliyuncs.com") {
        "DashScope OpenAI-compatible"
    } else if config.base.contains("api.openai.com") {
        "OpenAI"
    } else {
        "OpenAI-compatible"
    };
    json!({
        "configured": config.key.is_some(),
        "model": config.model,
        "ocrModel": config.ocr_model,
        "provider": provider,
        "baseConfigured": config.base_configured,
        "keySource": config.key_source,
        "cleanImageConfigured": clean_image.url.is_some() && clean_image.key.is_some(),
        "cleanImageProvider": "Generic Big LaMa",
        "cleanImageKeySource": clean_image.key_source,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalConfigRequest {
    base: String,
    model: String,
    key: String,
}

fn validate_setting(name: &str, value: &str, allow_empty: bool) -> Result<String, (u32, String)> {
    let value = value.trim();
    if value.contains(['\r', '\n', '\0']) {
        return Err((400, format!("{name} 包含非法换行或空字符")));
    }
    if !allow_empty && value.is_empty() {
        return Err((400, format!("{name} 不能为空")));
    }
    Ok(value.to_string())
}

fn quoted(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn merge_local_config(existing: &str, base: &str, model: &str, key: &str) -> String {
    let mut lines: Vec<&str> = existing
        .lines()
        .filter(|line| {
            let name = line.trim_start().split('=').next().unwrap_or("").trim();
            !matches!(name, "UNIPPT_AI_BASE" | "UNIPPT_AI_MODEL" | "UNIPPT_AI_KEY")
        })
        .collect();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let mut output = lines.join("\n");
    if !output.is_empty() {
        output.push_str("\n\n");
    }
    output.push_str("# Saved by the local UniPPT AI settings panel. Git-ignored.\n");
    output.push_str(&format!("UNIPPT_AI_BASE={}\n", quoted(base)));
    output.push_str(&format!("UNIPPT_AI_MODEL={}\n", quoted(model)));
    output.push_str(&format!("UNIPPT_AI_KEY={}\n", quoted(key)));
    output
}

pub fn save_local_config(body: &[u8]) -> Result<Value, (u32, String)> {
    let request: LocalConfigRequest = serde_json::from_slice(body)
        .map_err(|error| (400, format!("模型配置 JSON 无效：{error}")))?;
    let base = validate_setting("接口地址", &request.base, false)?;
    if !base.starts_with("https://") && !base.starts_with("http://") {
        return Err((400, "接口地址必须以 http:// 或 https:// 开头".into()));
    }
    let model = validate_setting("模型名称", &request.model, false)?;
    let key = validate_setting("API Key", &request.key, false)?;
    let path = local_config_path();
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let output = merge_local_config(&existing, &base, &model, &key);
    fs::write(&path, output).map_err(|error| {
        (
            500,
            format!("无法保存本机模型配置 {}：{error}", path.display()),
        )
    })?;
    Ok(json!({
        "ok": true,
        "configured": true,
        "model": model,
        "provider": if base.contains("dashscope.aliyuncs.com") { "DashScope OpenAI-compatible" } else { "OpenAI-compatible" },
        "keySource": "local-config",
    }))
}

pub fn chat_upstream(
    body: Vec<u8>,
) -> Result<Box<dyn Read + Send + Sync + 'static>, (u32, String)> {
    let mut payload: Value =
        serde_json::from_slice(&body).map_err(|error| (400, format!("请求 JSON 无效：{error}")))?;
    if !payload.is_object() {
        return Err((400, "请求体必须是 JSON 对象".into()));
    }
    let config = config();
    let Some(key) = config.key else {
        return Err((503, "内置模型通道尚未配置。请在启动服务前设置 UNIPPT_AI_KEY（可选 UNIPPT_AI_BASE / UNIPPT_AI_MODEL），或在 U AI 设置中填写自己的 API Key。".into()));
    };
    let model_missing = payload
        .get("model")
        .and_then(Value::as_str)
        .is_none_or(|value| value.trim().is_empty());
    if model_missing {
        payload["model"] = json!(config.model);
    }
    let request_model = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(&config.model);
    if supports_thinking_toggle(&config.base, request_model) {
        // Preserve the caller's task-specific latency policy. Repairs can
        // disable thinking; visual reconstruction can set a bounded budget.
        if !payload.get("enable_thinking").is_some_and(Value::is_boolean) {
            payload["enable_thinking"] = json!(true);
        }
    }
    payload["stream"] = json!(true);

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(12))
        .timeout_read(Duration::from_secs(60))
        .timeout_write(Duration::from_secs(30))
        .build();
    match agent
        .post(&format!(
            "{}/chat/completions",
            config.base.trim_end_matches('/')
        ))
        .set("Authorization", &format!("Bearer {key}"))
        .set("Content-Type", "application/json")
        .send_bytes(payload.to_string().as_bytes())
    {
        Ok(response) => Ok(Box::new(response.into_reader())),
        Err(ureq::Error::Status(code, response)) => {
            let detail = response.into_string().unwrap_or_default();
            let brief: String = detail.chars().take(800).collect();
            Err((code as u32, format!("AI 上游 HTTP {code}：{brief}")))
        }
        Err(error) => Err((502, format!("AI 上游连接失败：{error}"))),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OcrDetectRequest {
    image: String,
    width: u32,
    height: u32,
    #[serde(default)]
    attachment_name: String,
}

fn dashscope_ocr_url(base: &str) -> Option<String> {
    let base = base.trim_end_matches('/');
    let lower = base.to_ascii_lowercase();
    if !lower.contains("dashscope") && !lower.contains("aliyun") {
        return None;
    }
    if let Some(index) = lower.find("/compatible-mode/v1") {
        return Some(format!(
            "{}/api/v1/services/aigc/multimodal-generation/generation",
            &base[..index]
        ));
    }
    if let Some(index) = lower.find("/api/v1") {
        return Some(format!(
            "{}/api/v1/services/aigc/multimodal-generation/generation",
            &base[..index]
        ));
    }
    Some(format!(
        "{base}/api/v1/services/aigc/multimodal-generation/generation"
    ))
}

fn numeric_array(value: &Value, length: usize) -> Option<Vec<f64>> {
    let values = value.as_array()?;
    if values.len() != length {
        return None;
    }
    values.iter().map(Value::as_f64).collect()
}

fn normalized_ocr_regions(payload: &Value) -> Vec<Value> {
    payload
        .pointer("/output/choices/0/message/content/0/ocr_result/words_info")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|word| {
            let text = word.get("text")?.as_str()?.trim();
            let location = numeric_array(word.get("location")?, 8)?;
            if text.is_empty() {
                return None;
            }
            let rotate_rect = word
                .get("rotate_rect")
                .and_then(|value| numeric_array(value, 5));
            Some(json!({
                "text": text,
                "location": location,
                "rotateRect": rotate_rect,
            }))
        })
        .collect()
}

pub fn ocr_detect(body: &[u8]) -> Result<Value, (u32, String)> {
    let request: OcrDetectRequest = serde_json::from_slice(body)
        .map_err(|error| (400, format!("OCR 请求 JSON 无效：{error}")))?;
    if request.width == 0 || request.height == 0 {
        return Err((400, "OCR 请求缺少有效的原图尺寸".into()));
    }
    if !request.image.starts_with("data:image/") {
        return Err((400, "OCR 仅接受图片 Data URL".into()));
    }
    if request.image.len() > 18 * 1024 * 1024 {
        return Err((413, "OCR 图片超过 18 MiB 安全上限".into()));
    }
    let config = config();
    let Some(key) = config.key else {
        return Err((503, "OCR 通道尚未配置 DashScope API Key".into()));
    };
    let Some(url) = dashscope_ocr_url(&config.base) else {
        return Err((
            400,
            "当前 AI 接口不是 DashScope；请配置 UNIPPT_OCR_BASE 或使用 DashScope OCR".into(),
        ));
    };
    let payload = json!({
        "model": config.ocr_model,
        "input": {
            "messages": [{
                "role": "user",
                "content": [{
                    "image": request.image,
                    "min_pixels": 3072,
                    "max_pixels": 8388608,
                    "enable_rotate": false
                }]
            }]
        },
        "parameters": {
            "ocr_options": { "task": "advanced_recognition" }
        }
    });
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(12))
        .timeout_read(Duration::from_secs(120))
        .timeout_write(Duration::from_secs(30))
        .build();
    let upstream = match agent
        .post(&url)
        .set("Authorization", &format!("Bearer {key}"))
        .set("Content-Type", "application/json")
        .send_bytes(payload.to_string().as_bytes())
    {
        Ok(response) => {
            let text = response
                .into_string()
                .map_err(|error| (502, format!("OCR 上游响应读取失败：{error}")))?;
            serde_json::from_str::<Value>(&text)
                .map_err(|error| (502, format!("OCR 上游响应不是有效 JSON：{error}")))?
        }
        Err(ureq::Error::Status(code, response)) => {
            let detail = response.into_string().unwrap_or_default();
            let brief: String = detail.chars().take(800).collect();
            return Err((code as u32, format!("OCR 上游 HTTP {code}：{brief}")));
        }
        Err(error) => return Err((502, format!("OCR 上游连接失败：{error}"))),
    };
    let regions = normalized_ocr_regions(&upstream);
    if regions.is_empty() {
        let request_id = upstream
            .get("request_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        return Err((
            502,
            format!("OCR 未返回文字坐标（request_id={request_id}）"),
        ));
    }
    Ok(json!({
        "ok": true,
        "model": config.ocr_model,
        "attachmentName": request.attachment_name,
        "image": { "width": request.width, "height": request.height },
        "regionCount": regions.len(),
        "regions": regions,
        "requestId": upstream.get("request_id").cloned().unwrap_or(Value::Null),
        "usage": upstream.get("usage").cloned().unwrap_or(Value::Null),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CleanImageRequest {
    image: String,
    mask: String,
    #[serde(default = "default_mask_dilate")]
    mask_dilate: u8,
    #[serde(default)]
    feather_radius: u8,
    #[serde(default = "default_context_margin")]
    context_margin: u16,
    #[serde(default = "default_roi_mode")]
    roi_mode: String,
}

fn default_mask_dilate() -> u8 {
    6
}

fn default_context_margin() -> u16 {
    128
}

fn default_roi_mode() -> String {
    "auto".into()
}

fn decode_image_data_url(value: &str, label: &str) -> Result<(String, Vec<u8>), (u32, String)> {
    if value.len() > 24 * 1024 * 1024 {
        return Err((413, format!("{label} Data URL 超过 24 MiB 安全上限")));
    }
    let (metadata, encoded) = value
        .split_once(',')
        .ok_or_else(|| (400, format!("{label} 不是有效的 Data URL")))?;
    let media_type = metadata
        .strip_prefix("data:")
        .and_then(|metadata| metadata.split(';').next())
        .filter(|media_type| media_type.starts_with("image/"))
        .ok_or_else(|| (400, format!("{label} 仅接受图片 Data URL")))?;
    if !metadata.split(';').any(|part| part == "base64") {
        return Err((400, format!("{label} Data URL 必须使用 base64 编码")));
    }
    let bytes = BASE64
        .decode(encoded)
        .map_err(|error| (400, format!("{label} base64 无效：{error}")))?;
    if bytes.is_empty() || bytes.len() > 18 * 1024 * 1024 {
        return Err((413, format!("{label} 解码后为空或超过 18 MiB 安全上限")));
    }
    Ok((media_type.to_string(), bytes))
}

fn clean_image_endpoint(base: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.ends_with("/v1/inpaint") {
        base.to_string()
    } else if base.ends_with("/v1") {
        format!("{base}/inpaint")
    } else {
        format!("{base}/v1/inpaint")
    }
}

fn push_multipart_text(body: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes(),
    );
}

fn push_multipart_file(
    body: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    filename: &str,
    media_type: &str,
    bytes: &[u8],
) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {media_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n");
}

fn clean_image_multipart(
    request: &CleanImageRequest,
    image_type: &str,
    image: &[u8],
    mask_type: &str,
    mask: &[u8],
) -> (String, Vec<u8>) {
    let mut digest = Sha256::new();
    digest.update(image);
    digest.update(mask);
    let boundary = format!("----UniPPTCleanImage{:x}", digest.finalize());
    let mut body = Vec::with_capacity(image.len() + mask.len() + 1024);
    push_multipart_file(
        &mut body,
        &boundary,
        "image",
        "source.bin",
        image_type,
        image,
    );
    push_multipart_file(&mut body, &boundary, "mask", "mask.bin", mask_type, mask);
    push_multipart_text(
        &mut body,
        &boundary,
        "mask_dilate",
        &request.mask_dilate.to_string(),
    );
    push_multipart_text(
        &mut body,
        &boundary,
        "feather_radius",
        &request.feather_radius.to_string(),
    );
    push_multipart_text(
        &mut body,
        &boundary,
        "context_margin",
        &request.context_margin.to_string(),
    );
    push_multipart_text(&mut body, &boundary, "roi_mode", &request.roi_mode);
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (boundary, body)
}

fn timing_header(response: &ureq::Response, name: &str) -> Value {
    response
        .header(name)
        .and_then(|value| value.parse::<f64>().ok())
        .map(Value::from)
        .unwrap_or(Value::Null)
}

pub fn clean_image(body: &[u8]) -> Result<Value, (u32, String)> {
    let request: CleanImageRequest = serde_json::from_slice(body)
        .map_err(|error| (400, format!("擦字请求 JSON 无效：{error}")))?;
    if request.mask_dilate > 64 || request.feather_radius > 32 || request.context_margin > 2048 {
        return Err((400, "擦字参数超出安全范围".into()));
    }
    if !matches!(request.roi_mode.as_str(), "auto" | "full") {
        return Err((400, "roiMode 仅支持 auto 或 full".into()));
    }
    let (image_type, image) = decode_image_data_url(&request.image, "原图")?;
    let (mask_type, mask) = decode_image_data_url(&request.mask, "蒙版")?;
    let config = clean_image_config();
    let Some(url) = config.url else {
        return Err((503, "clean-image 服务地址尚未配置".into()));
    };
    let Some(key) = config.key else {
        return Err((503, "clean-image 服务密钥尚未配置".into()));
    };
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err((
            500,
            "clean-image 服务地址必须以 http:// 或 https:// 开头".into(),
        ));
    }
    let (boundary, multipart) =
        clean_image_multipart(&request, &image_type, &image, &mask_type, &mask);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(12))
        .timeout_read(Duration::from_secs(180))
        .timeout_write(Duration::from_secs(60))
        .build();
    let response = match agent
        .post(&clean_image_endpoint(&url))
        .set("Authorization", &format!("Bearer {key}"))
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        )
        .send_bytes(&multipart)
    {
        Ok(response) => response,
        Err(ureq::Error::Status(code, response)) => {
            let detail = response.into_string().unwrap_or_default();
            let brief: String = detail.chars().take(800).collect();
            return Err((
                code as u32,
                format!("clean-image 上游 HTTP {code}：{brief}"),
            ));
        }
        Err(error) => return Err((502, format!("clean-image 上游连接失败：{error}"))),
    };
    let request_id = response.header("X-Request-Id").unwrap_or("").to_string();
    let timings = json!({
        "modelMs": timing_header(&response, "X-Clean-Model-Ms"),
        "workerMs": timing_header(&response, "X-Model-Inference-Ms"),
        "totalMs": timing_header(&response, "X-Clean-Total-Ms"),
        "decodeMs": timing_header(&response, "X-Clean-Decode-Ms"),
        "encodeMs": timing_header(&response, "X-Clean-Encode-Ms"),
    });
    let roi = response.header("X-Clean-Roi").unwrap_or("").to_string();
    let mut repaired = Vec::new();
    response
        .into_reader()
        .take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut repaired)
        .map_err(|error| (502, format!("clean-image 响应读取失败：{error}")))?;
    if repaired.is_empty() || repaired.len() > 32 * 1024 * 1024 {
        return Err((502, "clean-image 返回空图片或超过 32 MiB 安全上限".into()));
    }
    Ok(json!({
        "ok": true,
        "image": format!("data:image/png;base64,{}", BASE64.encode(repaired)),
        "model": "generic-big-lama",
        "cleaning": {
            "strategy": "big-lama-roi-v1",
            "maskDilate": request.mask_dilate,
            "featherRadius": request.feather_radius,
            "contextMargin": request.context_margin,
            "roiMode": request.roi_mode,
            "roi": roi,
            "requestId": request_id,
            "timings": timings,
        }
    }))
}

fn supports_thinking_toggle(base: &str, model: &str) -> bool {
    let base = base.to_ascii_lowercase();
    let model = model.to_ascii_lowercase();
    base.contains("dashscope") || base.contains("aliyun") || model.starts_with("qwen")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_never_exposes_a_key() {
        let text = status().to_string();
        assert!(!text.contains("Bearer"));
        assert!(!text.contains("apiKey"));
        assert!(!text.contains("secret"));
    }

    #[test]
    fn qwen_compatible_routes_keep_thinking_enabled() {
        assert!(supports_thinking_toggle(
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "qwen3.7-flash"
        ));
        assert!(supports_thinking_toggle(
            "https://example.test/v1",
            "qwen-plus"
        ));
        assert!(!supports_thinking_toggle(
            "https://api.openai.com/v1",
            "gpt-5"
        ));
    }

    #[test]
    fn dashscope_compatible_base_maps_to_native_ocr_generation() {
        assert_eq!(
            dashscope_ocr_url("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            Some("https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation".into())
        );
        assert_eq!(dashscope_ocr_url("https://api.openai.com/v1"), None);
    }

    #[test]
    fn native_ocr_words_are_normalized_without_losing_polygons() {
        let payload = json!({
            "output": { "choices": [{ "message": { "content": [{ "ocr_result": { "words_info": [
                { "text": "智启未来", "location": [52, 54, 250, 57, 249, 106, 52, 103], "rotate_rect": [150, 80, 49, 197, -89] },
                { "text": "", "location": [0, 0, 1, 0, 1, 1, 0, 1] }
            ] } }] } }] }
        });
        let regions = normalized_ocr_regions(&payload);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0]["text"], "智启未来");
        assert_eq!(regions[0]["location"].as_array().unwrap().len(), 8);
        assert_eq!(regions[0]["rotateRect"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn invalid_chat_body_is_rejected_before_network_access() {
        let error = chat_upstream(b"[]".to_vec()).err().expect("invalid body");
        assert_eq!(error.0, 400);
    }

    #[test]
    fn local_config_merge_preserves_unrelated_settings_and_never_logs_the_key() {
        let text = merge_local_config(
            "UNIPPT_PORT=9000\nUNIPPT_AI_KEY=old\n# keep me\n",
            DEFAULT_BASE,
            DEFAULT_MODEL,
            "new-secret",
        );
        assert!(text.contains("UNIPPT_PORT=9000"));
        assert!(text.contains("# keep me"));
        assert_eq!(text.matches("UNIPPT_AI_KEY=").count(), 1);
        assert!(!status().to_string().contains("new-secret"));
    }

    #[test]
    fn clean_image_endpoint_accepts_roots_and_versioned_bases() {
        assert_eq!(
            clean_image_endpoint("https://example.test"),
            "https://example.test/v1/inpaint"
        );
        assert_eq!(
            clean_image_endpoint("https://example.test/v1/"),
            "https://example.test/v1/inpaint"
        );
        assert_eq!(
            clean_image_endpoint("https://example.test/v1/inpaint"),
            "https://example.test/v1/inpaint"
        );
    }

    #[test]
    fn clean_image_multipart_contains_files_and_parameters() {
        let request = CleanImageRequest {
            image: String::new(),
            mask: String::new(),
            mask_dilate: 6,
            feather_radius: 0,
            context_margin: 128,
            roi_mode: "auto".into(),
        };
        let (_, body) = clean_image_multipart(
            &request,
            "image/png",
            b"source-bytes",
            "image/png",
            b"mask-bytes",
        );
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("name=\"image\""));
        assert!(text.contains("source-bytes"));
        assert!(text.contains("name=\"mask\""));
        assert!(text.contains("mask-bytes"));
        assert!(text.contains("name=\"mask_dilate\"\r\n\r\n6"));
        assert!(text.contains("name=\"roi_mode\"\r\n\r\nauto"));
    }
}
