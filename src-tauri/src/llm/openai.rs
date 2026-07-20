use futures::{StreamExt, stream};
use reqwest::Client;
use serde_json::{Value, json};

use super::types::{ChatRequest, MessageRole, ProviderKind, StreamEvent, ToolCall};

pub async fn stream_chat(req: ChatRequest) -> Result<super::ChatStream, String> {
    let is_ollama = req.settings.provider == ProviderKind::Ollama;
    let api_key = req.settings.api_key.trim();

    if !is_ollama && api_key.is_empty() {
        return Err("API key is empty".into());
    }

    let base = req
        .settings
        .base_url
        .as_deref()
        .unwrap_or(if is_ollama {
            "http://localhost:11434/v1"
        } else {
            "https://api.openai.com/v1"
        })
        .trim_end_matches('/');

    let body = build_body(&req)?;
    let client = Client::new();
    let mut request = client
        .post(format!("{base}/chat/completions"))
        .header("content-type", "application/json");

    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {api_key}"));
    }

    let response = request
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI-compatible request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI-compatible error {status}: {text}"));
    }

    Ok(Box::pin(parse_sse_stream(response.bytes_stream())))
}

fn build_body(req: &ChatRequest) -> Result<Value, String> {
    let mut messages = vec![json!({"role": "system", "content": req.system})];

    for msg in &req.messages {
        match msg.role {
            MessageRole::System => continue,
            MessageRole::User => {
                messages.push(json!({"role": "user", "content": msg.content}));
            }
            MessageRole::Assistant => {
                let mut m = json!({"role": "assistant", "content": msg.content});
                if let Some(tool_calls) = &msg.tool_calls {
                    let calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments,
                                }
                            })
                        })
                        .collect();
                    m["tool_calls"] = Value::Array(calls);
                }
                messages.push(m);
            }
            MessageRole::Tool => {
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": msg.tool_call_id,
                    "content": msg.content,
                }));
            }
        }
    }

    let tools: Vec<Value> = req
        .tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema,
                }
            })
        })
        .collect();

    Ok(json!({
        "model": req.settings.model,
        "stream": true,
        "messages": messages,
        "tools": tools,
    }))
}

struct OpenAiParser {
    buffer: String,
    tool_calls: std::collections::HashMap<usize, ToolCall>,
    done: bool,
}

impl OpenAiParser {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            tool_calls: std::collections::HashMap::new(),
            done: false,
        }
    }

    fn push_chunk(&mut self, chunk: &[u8]) -> Vec<Result<StreamEvent, String>> {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));
        let mut out = Vec::new();

        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].trim().to_string();
            self.buffer = self.buffer[pos + 1..].to_string();
            out.extend(self.parse_line(&line));
        }
        out
    }

    fn finish(&mut self) -> Vec<Result<StreamEvent, String>> {
        if self.done {
            return Vec::new();
        }
        self.done = true;
        let mut out = Vec::new();
        if !self.tool_calls.is_empty() {
            let mut calls: Vec<ToolCall> = self.tool_calls.drain().map(|(_, v)| v).collect();
            calls.sort_by(|a, b| a.id.cmp(&b.id));
            out.push(Ok(StreamEvent::ToolCalls(calls)));
        }
        out.push(Ok(StreamEvent::Done));
        out
    }

    fn parse_line(&mut self, line: &str) -> Vec<Result<StreamEvent, String>> {
        let mut out = Vec::new();
        if !line.starts_with("data:") {
            return out;
        }
        let data = line[5..].trim();
        if data.is_empty() || data == "[DONE]" {
            if data == "[DONE]" {
                if !self.tool_calls.is_empty() {
                    let mut calls: Vec<ToolCall> = self.tool_calls.drain().map(|(_, v)| v).collect();
                    calls.sort_by(|a, b| a.id.cmp(&b.id));
                    out.push(Ok(StreamEvent::ToolCalls(calls)));
                }
                out.push(Ok(StreamEvent::Done));
                self.done = true;
            }
            return out;
        }

        let Ok(value) = serde_json::from_str::<Value>(data) else {
            return out;
        };

        if let Some(err) = value.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("OpenAI stream error")
                .to_string();
            out.push(Err(msg));
            self.done = true;
            return out;
        }

        let choice = value
            .get("choices")
            .and_then(|c| c.get(0));

        if let Some(delta) = choice.and_then(|c| c.get("delta")) {
            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                if !content.is_empty() {
                    out.push(Ok(StreamEvent::TextDelta(content.to_string())));
                }
            }

            if let Some(calls) = delta.get("tool_calls").and_then(|c| c.as_array()) {
                for call in calls {
                    let idx = call.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                    let entry = self
                        .tool_calls
                        .entry(idx)
                        .or_insert_with(|| ToolCall {
                            id: String::new(),
                            name: String::new(),
                            arguments: String::new(),
                        });

                    if let Some(id) = call.get("id").and_then(|i| i.as_str()) {
                        entry.id = id.to_string();
                    }
                    if let Some(name) = call
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        entry.name = name.to_string();
                    }
                    if let Some(args) = call
                        .get("function")
                        .and_then(|f| f.get("arguments"))
                        .and_then(|a| a.as_str())
                    {
                        entry.arguments.push_str(args);
                    }
                }
            }
        }

        if let Some(finish) = choice.and_then(|c| c.get("finish_reason")).and_then(|f| f.as_str()) {
            if finish == "tool_calls" || finish == "stop" {
                if !self.tool_calls.is_empty() {
                    let mut calls: Vec<ToolCall> = self.tool_calls.drain().map(|(_, v)| v).collect();
                    calls.sort_by(|a, b| a.id.cmp(&b.id));
                    out.push(Ok(StreamEvent::ToolCalls(calls)));
                }
                out.push(Ok(StreamEvent::Done));
                self.done = true;
            }
        }

        out
    }
}

fn parse_sse_stream(
    byte_stream: impl futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> impl futures::Stream<Item = Result<StreamEvent, String>> + Send {
    stream::unfold(
        (Box::pin(byte_stream), OpenAiParser::new(), Vec::new(), false),
        |(mut bytes, mut parser, mut pending, finished)| async move {
            if let Some(event) = pending.pop() {
                return Some((event, (bytes, parser, pending, finished)));
            }
            if finished {
                return None;
            }

            loop {
                match bytes.next().await {
                    Some(Ok(chunk)) => {
                        pending = parser.push_chunk(&chunk);
                        pending.reverse();
                        if let Some(event) = pending.pop() {
                            return Some((event, (bytes, parser, pending, false)));
                        }
                    }
                    Some(Err(e)) => {
                        return Some((
                            Err(format!("stream read error: {e}")),
                            (bytes, parser, pending, true),
                        ));
                    }
                    None => {
                        pending = parser.finish();
                        pending.reverse();
                        if let Some(event) = pending.pop() {
                            let still_pending = !pending.is_empty();
                            return Some((event, (bytes, parser, pending, !still_pending)));
                        }
                        return None;
                    }
                }
            }
        },
    )
}
