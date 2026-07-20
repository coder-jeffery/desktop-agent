pub mod anthropic;
pub mod openai;
pub mod types;

use futures::Stream;
use std::pin::Pin;

use types::{ChatRequest, StreamEvent};

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<StreamEvent, String>> + Send>>;

pub async fn stream_chat(req: ChatRequest) -> Result<ChatStream, String> {
    match req.settings.provider {
        types::ProviderKind::Anthropic => anthropic::stream_chat(req).await,
        types::ProviderKind::OpenAiCompatible | types::ProviderKind::Ollama => {
            openai::stream_chat(req).await
        }
    }
}
