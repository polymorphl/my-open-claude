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
use crate::core::workspace::Workspace;

pub use error::{ChatError, map_api_error};
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
/// Sync required so futures holding &OnProgress across await points are Send.
pub type OnProgress = Box<dyn Fn(&str) + Send + Sync>;

/// Callback for each streamed content chunk (text only).
/// Sync required so futures holding &OnContentChunk across await points are Send.
pub type OnContentChunk = Box<dyn Fn(&str) + Send + Sync>;

/// Optional callbacks for chat: progress, streaming, cancellation.
#[derive(Default)]
pub struct ChatOptions {
    /// Called when progress events occur (e.g. "Calling API...", "→ Bash: ls").
    pub on_progress: Option<OnProgress>,
    /// Called for each streamed content chunk (text only).
    pub on_content_chunk: Option<OnContentChunk>,
    /// When cancelled, the request is aborted.
    pub cancel_token: Option<CancellationToken>,
}

/// Parameters for starting a new chat.
pub struct ChatRequest<'a> {
    /// API and application configuration.
    pub config: &'a Config,
    /// Model ID (e.g. "anthropic/claude-haiku-4.5").
    pub model: &'a str,
    /// User prompt.
    pub prompt: &'a str,
    /// Mode: "Ask" (read-only tools) or "Build" (all tools).
    pub mode: &'a str,
    /// Model context window length (tokens).
    pub context_length: u64,
    /// Callback for destructive command confirmation (CLI mode). TUI uses popup instead.
    pub confirm_destructive: Option<crate::core::confirm::ConfirmDestructive>,
    /// Previous conversation messages to resume from (API format).
    pub previous_messages: Option<Vec<Value>>,
    /// Optional progress, streaming, and cancellation callbacks.
    pub options: ChatOptions,
    /// Workspace root, project type, optional AGENTS.md content.
    pub workspace: &'a Workspace,
    /// Tools to make available to the agent (injected by caller).
    pub tools_list: &'a [Box<dyn tools::Tool>],
    /// Tool definitions for the API (must match tools_list order).
    pub tools_defs: &'a [Value],
}

/// Run an agent loop that:
/// - starts with the user's prompt (and optional previous conversation)
/// - repeatedly calls the model
/// - executes any requested tools (except Write/Bash in Ask mode)
/// - feeds tool results back to the model
/// - stops when the model responds without tool calls
pub async fn chat(req: ChatRequest<'_>) -> Result<ChatResult, ChatError> {
    let client = Client::with_config(req.config.openai_config.clone());

    let root = req.workspace.root.display().to_string();
    let project_type = req
        .workspace
        .project_type
        .map(|pt| pt.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut content = format!(
        "Respond in the same language as the user. If they write in French, respond in French; if in English, respond in English; match their language.\n\nWorkspace root: {}\nProject type: {}\nUse the workspace root as the default base path for Read, Write, Grep, ListDir, Glob, and Edit when the user does not specify a path.",
        root, project_type
    );

    if let Some(ref agent_md) = req.workspace.agent_md {
        content.push_str("\n\n--- Project context (AGENTS.md) ---\n");
        content.push_str(agent_md);
        content.push_str("\n---");
    }

    let system_msg = json!({
        "role": "system",
        "content": content
    });

    let mut messages: Vec<Value> = req.previous_messages.unwrap_or_default();
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
        "content": req.prompt,
    }));
    let mut messages = Arc::new(messages);
    let mut tool_log = Arc::new(Vec::<String>::new());
    let confirm_destructive = req.confirm_destructive;

    agent_loop::run_agent_loop(
        agent_loop::AgentLoopParams {
            client: &client,
            model: req.model,
            context_length: req.context_length,
            tools_defs: req.tools_defs,
            tools_list: req.tools_list,
            messages: &mut messages,
            tool_log: &mut tool_log,
            mode: req.mode,
        },
        agent_loop::AgentLoopCallbacks {
            confirm_destructive: &confirm_destructive,
            on_progress: req.options.on_progress.as_deref(),
            on_content_chunk: req.options.on_content_chunk.as_deref(),
            cancel_token: req.options.cancel_token.as_ref(),
        },
    )
    .await
}

/// Resume the chat loop after user confirmed or cancelled a destructive command.
///
/// Call when the user answered y/n to the destructive command confirmation popup.
///
/// # Arguments
///
/// * `state` - Internal state from `ChatResult::NeedsConfirmation`, required to continue the loop.
/// * `confirmed` - `true` if user accepted, `false` if cancelled (sends "Command cancelled" to model).
pub async fn chat_resume(
    config: &Config,
    model: &str,
    context_length: u64,
    state: ConfirmState,
    confirmed: bool,
    tools_list: &[Box<dyn tools::Tool>],
    options: impl Into<ChatOptions>,
) -> Result<ChatResult, ChatError> {
    let opts = options.into();
    let client = Client::with_config(config.openai_config.clone());

    let bash_tool = tools::BashTool;
    let result = if confirmed {
        tool_execution::tool_result_string(
            bash_tool.execute(&json!({ "command": state.command })),
            "Bash",
        )
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

    agent_loop::run_agent_loop(
        agent_loop::AgentLoopParams {
            client: &client,
            model,
            context_length,
            tools_defs: &tools_defs,
            tools_list,
            messages: &mut messages,
            tool_log: &mut tool_log,
            mode: &state.mode,
        },
        agent_loop::AgentLoopCallbacks {
            confirm_destructive: &None,
            on_progress: opts.on_progress.as_deref(),
            on_content_chunk: opts.on_content_chunk.as_deref(),
            cancel_token: opts.cancel_token.as_ref(),
        },
    )
    .await
}
