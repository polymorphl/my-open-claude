//! Agent loop: chat with tool calling, streaming, and destructive command confirmation.

mod agent_loop;
pub(crate) mod context;
mod error;
mod stream;
mod tool_execution;

use async_openai::Client;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::core::config::Config;
use crate::core::tools;
use crate::core::tools::Tool;

pub use error::{map_api_error, ChatError};
pub use stream::TokenUsage;
#[allow(unused_imports)]
pub use tool_execution::is_ask_mode;

/// Result of a chat turn. Either complete, or needs user confirmation for a destructive command.
#[derive(Debug)]
pub enum ChatResult {
    Complete {
        content: String,
        tool_log: Vec<String>,
        messages: Vec<Value>,
        usage: TokenUsage,
    },
    /// Destructive command pending; caller must show confirmation UI then call `chat_resume`.
    NeedsConfirmation {
        command: String,
        state: ConfirmState,
    },
}

/// Internal state to resume the chat loop after user confirms or cancels.
#[derive(Debug)]
pub struct ConfirmState {
    pub(crate) messages: Arc<Vec<Value>>,
    pub(crate) tool_log: Arc<Vec<String>>,
    pub(crate) tool_call_id: String,
    pub(crate) mode: String,
    pub(crate) tools: Vec<Value>,
    pub(crate) command: String,
}

/// Callback for progress updates during chat (e.g. "Calling API...", "→ Bash: ls").
pub type OnProgress = Box<dyn Fn(&str) + Send>;

/// Callback for each streamed content chunk (text only).
pub type OnContentChunk = Box<dyn Fn(&str) + Send>;

/// Run an agent loop that:
/// - starts with the user's prompt (and optional previous conversation)
/// - repeatedly calls the model
/// - executes any requested tools (except Write/Bash in Ask mode)
/// - feeds tool results back to the model
/// - stops when the model responds without tool calls
pub async fn chat(
    config: &Config,
    model: &str,
    prompt: &str,
    mode: &str,
    context_length: u64,
    confirm_destructive: Option<crate::core::confirm::ConfirmDestructive>,
    previous_messages: Option<Vec<Value>>,
    on_progress: Option<OnProgress>,
    on_content_chunk: Option<OnContentChunk>,
    cancel_token: Option<CancellationToken>,
) -> Result<ChatResult, ChatError> {
    let client = Client::with_config(config.openai_config.clone());

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let system_msg = json!({
        "role": "system",
        "content": format!("Current working directory: {}", cwd)
    });

    let mut messages: Vec<Value> = previous_messages.unwrap_or_default();
    if messages.is_empty()
        || messages
            .first()
            .and_then(|m| m.get("role").and_then(|r| r.as_str()))
            != Some("system")
    {
        messages.insert(0, system_msg);
    }
    messages.push(json!({
        "role": "user",
        "content": prompt,
    }));
    let mut messages = Arc::new(messages);
    let mut tool_log = Arc::new(Vec::<String>::new());

    agent_loop::run_agent_loop(
        &client,
        config,
        model,
        context_length,
        &tools::definitions(),
        &tools::all(),
        &mut messages,
        &mut tool_log,
        mode,
        &confirm_destructive,
        on_progress.as_deref(),
        on_content_chunk.as_deref(),
        cancel_token.as_ref(),
    )
    .await
}

/// Resume the chat loop after user confirmed or cancelled a destructive command.
pub async fn chat_resume(
    config: &Config,
    model: &str,
    context_length: u64,
    state: ConfirmState,
    confirmed: bool,
    on_progress: Option<OnProgress>,
    on_content_chunk: Option<OnContentChunk>,
    cancel_token: Option<CancellationToken>,
) -> Result<ChatResult, ChatError> {
    let client = Client::with_config(config.openai_config.clone());

    let bash_tool = tools::BashTool;
    let result = if confirmed {
        bash_tool
            .execute(&json!({ "command": state.command }))
            .unwrap_or_else(|e| format!("Error: {}", e))
    } else {
        "Command cancelled (destructive command not confirmed).".to_string()
    };

    let mut messages = state.messages;
    Arc::make_mut(&mut messages).push(json!({
        "role": "tool",
        "tool_call_id": state.tool_call_id,
        "content": result,
    }));

    let mut tool_log = state.tool_log;
    let tools_defs = state.tools;
    let tools_list = tools::all();

    agent_loop::run_agent_loop(
        &client,
        config,
        model,
        context_length,
        &tools_defs,
        &tools_list,
        &mut messages,
        &mut tool_log,
        &state.mode,
        &None,
        on_progress.as_deref(),
        on_content_chunk.as_deref(),
        cancel_token.as_ref(),
    )
    .await
}
