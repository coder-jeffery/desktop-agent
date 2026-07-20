use reqwest::multipart;
use serde::Deserialize;

use crate::llm::types::ProviderKind;

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: Option<String>,
}

/// Transcribe WAV audio via OpenAI-compatible `/audio/transcriptions`.
///
/// We intentionally avoid Apple's `SFSpeechRecognizer`: when `tauri dev` is
/// launched from Cursor/VS Code, TCC treats Cursor as the responsible process
/// and aborts the app even if `NSSpeechRecognitionUsageDescription` is
/// embedded in the binary.
pub async fn transcribe_wav(
    wav_bytes: &[u8],
    provider: ProviderKind,
    api_key: &str,
    base_url: Option<&str>,
    locale: &str,
) -> Result<String, String> {
    if wav_bytes.len() < 44 {
        return Err("录音太短或无效".into());
    }

    let is_ollama = provider == ProviderKind::Ollama;
    let api_key = api_key.trim();
    if !is_ollama && api_key.is_empty() {
        return Err(
            "语音转写需要 API Key。请在设置中配置支持 Whisper 的 OpenAI 兼容服务（如 OpenAI / Groq / SiliconFlow）。"
                .into(),
        );
    }

    let base = base_url
        .unwrap_or(if is_ollama {
            "http://localhost:11434/v1"
        } else {
            "https://api.openai.com/v1"
        })
        .trim_end_matches('/');

    let language = whisper_language(locale);
    let model = whisper_model(provider);

    let part = multipart::Part::bytes(wav_bytes.to_vec())
        .file_name("speech.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let mut form = multipart::Form::new()
        .part("file", part)
        .text("model", model)
        .text("response_format", "json");
    if let Some(lang) = language {
        form = form.text("language", lang);
    }

    let client = reqwest::Client::new();
    let mut request = client.post(format!("{base}/audio/transcriptions"));
    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {api_key}"));
    }

    let response = request
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("语音转写请求失败: {e}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("读取转写响应失败: {e}"))?;

    if !status.is_success() {
        let hint = if status.as_u16() == 404 {
            " 当前服务可能不支持 /audio/transcriptions，请改用 OpenAI、Groq 或 SiliconFlow 等 Whisper 接口。"
        } else {
            ""
        };
        return Err(format!("语音转写失败 ({status}): {body}{hint}"));
    }

    if let Ok(parsed) = serde_json::from_str::<TranscriptionResponse>(&body) {
        let text = parsed.text.unwrap_or_default().trim().to_string();
        if text.is_empty() {
            return Err("未识别到语音内容".into());
        }
        return Ok(text);
    }

    let text = body.trim().trim_matches('"').to_string();
    if text.is_empty() {
        return Err("未识别到语音内容".into());
    }
    Ok(text)
}

fn whisper_language(locale: &str) -> Option<String> {
    let lower = locale.to_ascii_lowercase();
    if lower.starts_with("zh") {
        Some("zh".into())
    } else if lower.starts_with("en") {
        Some("en".into())
    } else if lower.len() >= 2 {
        Some(lower[..2].to_string())
    } else {
        None
    }
}

fn whisper_model(provider: ProviderKind) -> String {
    match provider {
        ProviderKind::Ollama => "whisper".into(),
        _ => "whisper-1".into(),
    }
}
