//! Spawns chat requests in a background thread with progress/stream/result channels.

use std::sync::mpsc;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio_util::sync::CancellationToken;

use serde_json::Value;

use crate::core::config::Config;
use crate::core::llm;

use super::PendingChat;

/// Spawn a new chat request. Returns PendingChat with channels for progress, stream, and result.
pub fn spawn_chat(
    rt: &Arc<Runtime>,
    config: Arc<Config>,
    model_id: String,
    prompt: String,
    mode: String,
    prev_messages: Option<Vec<Value>>,
) -> PendingChat {
    let (progress_tx, progress_rx) = mpsc::channel();
    let (stream_tx, stream_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let context_length = crate::core::models::resolve_context_length(&model_id);
    let rt_clone = Arc::clone(rt);

    std::thread::spawn(move || {
        let on_progress: llm::OnProgress = Box::new(move |s| {
            let _ = progress_tx.send(s.to_string());
        });
        let on_content_chunk: llm::OnContentChunk = Box::new(move |s| {
            let _ = stream_tx.send(s.to_string());
        });
        let result = rt_clone.block_on(llm::chat(
            config.as_ref(),
            &model_id,
            &prompt,
            &mode,
            context_length,
            None,
            prev_messages,
            Some(on_progress),
            Some(on_content_chunk),
            Some(cancel_token_clone),
        ));
        let _ = result_tx.send(result);
    });

    PendingChat {
        progress_rx,
        stream_rx,
        result_rx,
        cancel_token,
    }
}

/// Spawn chat_resume after user confirmed or cancelled a destructive command.
pub fn spawn_chat_resume(
    rt: &Arc<Runtime>,
    config: Arc<Config>,
    model_id: String,
    state: llm::ConfirmState,
    confirmed: bool,
) -> PendingChat {
    let (progress_tx, progress_rx) = mpsc::channel();
    let (stream_tx, stream_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let context_length = crate::core::models::resolve_context_length(&model_id);
    let rt_clone = Arc::clone(rt);

    std::thread::spawn(move || {
        let on_progress: llm::OnProgress = Box::new(move |s| {
            let _ = progress_tx.send(s.to_string());
        });
        let on_content_chunk: llm::OnContentChunk = Box::new(move |s| {
            let _ = stream_tx.send(s.to_string());
        });
        let result = rt_clone.block_on(llm::chat_resume(
            config.as_ref(),
            &model_id,
            context_length,
            state,
            confirmed,
            Some(on_progress),
            Some(on_content_chunk),
            Some(cancel_token_clone),
        ));
        let _ = result_tx.send(result);
    });

    PendingChat {
        progress_rx,
        stream_rx,
        result_rx,
        cancel_token,
    }
}
