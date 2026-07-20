use futures::{StreamExt, stream};
use reqwest::Client;
use serde_json::{Value, json};

use super::types::{
    ChatRequest, StreamEvent, ToolCall, ToolDefinition,
};

pub async fn stream_chat(req: ChatRequest) -> Result<super::ChatStream, String> {
    let api_key = req.settings.api_key.trim();
    if api_key.is_empty() {
        return Err("Anthropic API key is empty".into());
    }

    let body = build_body(&req)?;
    let client = Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Anthropic request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Anthropic error {status}: {text}"));
    }

    Ok(Box::pin(parse_sse_stream(response.bytes_stream())))
}

fn build_body(req: &ChatRequest) -> Result<Value, String> {
    use super::types::MessageRole;

    let mut messages = Vec::new();
    for msg in &req.messages {
        match msg.role {
            MessageRole::System => continue,
            MessageRole::User => {
                messages.push(json!({
                    "role": "user",
                    "content": msg.content,
                }));
            }
            MessageRole::Assistant => {
                let mut content: Vec<Value> = Vec::new();
                if !msg.content.is_empty() {
                    content.push(json!({"type": "text", "text": msg.content}));
                }
                if let Some(tool_calls) = &msg.tool_calls {
                    for tc in tool_calls {
                        let input: Value = serde_json::from_str(&tc.arguments)
                            .unwrap_or_else(|_| json!({}));
                        content.push(json!({
                            "type": "tool_use",
                            "id": tc.id,
                            "name": tc.name,
                            "input": input,
                        }));
                    }
                }
                messages.push(json!({"role": "assistant", "content": content}));
            }
            MessageRole::Tool => {
                messages.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": msg.tool_call_id.clone().unwrap_or_default(),
                        "content": msg.content,
                    }]
                }));
            }
        }
    }

    messages = merge_consecutive_user_messages(messages);

    let tools: Vec<Value> = req
        .tools
        .iter()
        .map(|t: &ToolDefinition| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect();

    Ok(json!({
        "model": req.settings.model,
        "max_tokens": 8192,
        "stream": true,
        "system": req.system,
        "messages": messages,
        "tools": tools,
    }))
}

fn merge_consecutive_user_messages(messages: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for msg in messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role == "user" {
            if let Some(last) = out.last_mut() {
                if last.get("role").and_then(|r| r.as_str()) == Some("user") {
                    let mut content = match last.get_mut("content") {
                        Some(Value::Array(arr)) => arr.clone(),
                        Some(Value::String(s)) => vec![json!({"type": "text", "text": s})],
                        _ => Vec::new(),
                    };
                    match msg.get("content") {
                        Some(Value::Array(arr)) => content.extend(arr.clone()),
                        Some(Value::String(s)) => {
                            content.push(json!({"type": "text", "text": s}));
                        }
                        _ => {}
                    }
                    last["content"] = Value::Array(content);
                    continue;
                }
            }
        }
        out.push(msg);
    }
    out
}

struct SseParser {
    buffer: String,
    current_tool: Option<(String, String, String)>,
    tool_calls: Vec<ToolCall>,
    done: bool,
}

impl SseParser {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            current_tool: None,
            tool_calls: Vec::new(),
            done: false,
        }
    }

    fn push_chunk(&mut self, chunk: &[u8]) -> Vec<Result<StreamEvent, String>> {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));
        let mut out = Vec::new();

        while let Some(pos) = self.buffer.find("\n\n") {
            let frame = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();
            out.extend(self.parse_frame(&frame));
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
            out.push(Ok(StreamEvent::ToolCalls(std::mem::take(&mut self.tool_calls))));
        }
        out.push(Ok(StreamEvent::Done));
        out
    }

    fn parse_frame(&mut self, frame: &str) -> Vec<Result<StreamEvent, String>> {
        let mut out = Vec::new();
        for line in frame.lines() {
            let line = line.trim();
            if !line.starts_with("data:") {
                continue;
            }
            let data = line[5..].trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            let event_type = value.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match event_type {
                "content_block_start" => {
                    if let Some(block) = value.get("content_block") {
                        if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                            let id = block
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let name = block
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            self.current_tool = Some((id, name, String::new()));
                        }
                    }
                }
                "content_block_delta" => {
                    if let Some(delta) = value.get("delta") {
                        match delta.get("type").and_then(|t| t.as_str()) {
                            Some("text_delta") => {
                                if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                                    out.push(Ok(StreamEvent::TextDelta(text.to_string())));
                                }
                            }
                            Some("input_json_delta") => {
                                if let Some(partial) =
                                    delta.get("partial_json").and_then(|t| t.as_str())
                                {
                                    if let Some((_, _, args)) = self.current_tool.as_mut() {
                                        args.push_str(partial);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "content_block_stop" => {
                    if let Some((id, name, args)) = self.current_tool.take() {
                        self.tool_calls.push(ToolCall {
                            id,
                            name,
                            arguments: if args.is_empty() { "{}".into() } else { args },
                        });
                    }
                }
                "message_stop" => {
                    if !self.tool_calls.is_empty() {
                        out.push(Ok(StreamEvent::ToolCalls(std::mem::take(
                            &mut self.tool_calls,
                        ))));
                    }
                    out.push(Ok(StreamEvent::Done));
                    self.done = true;
                }
                "error" => {
                    let msg = value
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("Anthropic stream error")
                        .to_string();
                    out.push(Err(msg));
                    self.done = true;
                }
                _ => {}
            }
        }
        out
    }
}

fn parse_sse_stream(
    byte_stream: impl futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> impl futures::Stream<Item = Result<StreamEvent, String>> + Send {
    stream::unfold(
        (Box::pin(byte_stream), SseParser::new(), Vec::new(), false),
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
                            return Some((
                                event,
                                (bytes, parser, pending, !still_pending),
                            ));
                        }
                        return None;
                    }
                }
            }
        },
    )
}
