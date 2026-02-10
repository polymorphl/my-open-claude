use async_openai::config::OpenAIConfig;
use async_openai::Client;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::confirm::ConfirmDestructive;
use crate::core::config::Config;
use crate::core::tools;
use crate::core::tools::Tool;

/// Errors from the chat/agent pipeline.
#[derive(Debug)]
pub enum ChatError {
    ApiAuth(String),
    ApiMessage(String),
    ToolArgs { tool: String, source: serde_json::Error },
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatError::ApiAuth(msg) => write!(f, "{}", msg),
            ChatError::ApiMessage(msg) => write!(f, "API error: {}", msg),
            ChatError::ToolArgs { tool, source } => {
                write!(f, "Invalid tool arguments for {}: {}", tool, source)
            }
            ChatError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ChatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ChatError::ToolArgs { source, .. } => Some(source),
            ChatError::Other(e) => e.source(),
            _ => None,
        }
    }
}

/// Interaction mode: "Ask" = explanations only (no write/bash), "Build" = all tools.
pub fn is_ask_mode(mode: &str) -> bool {
    mode.eq_ignore_ascii_case("ask")
}

/// Result of a chat turn. Either complete, or needs user confirmation for a destructive command.
#[derive(Debug)]
pub enum ChatResult {
    Complete {
        content: String,
        tool_log: Vec<String>,
        messages: Vec<Value>,
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
    pub(super) messages: Arc<Vec<Value>>,
    pub(super) tool_log: Arc<Vec<String>>,
    pub(super) tool_call_id: String,
    pub(super) mode: String,
    pub(super) tools: Vec<Value>,
    pub(super) command: String,
}

fn make_complete(
    content: &str,
    tool_log: &[String],
    messages: &[Value],
) -> ChatResult {
    ChatResult::Complete {
        content: content.to_string(),
        tool_log: tool_log.to_vec(),
        messages: messages.to_vec(),
    }
}

/// Callback for progress updates during chat (e.g. "Calling API...", "→ Bash: ls").
pub type OnProgress = Box<dyn Fn(&str) + Send>;

/// Run an agent loop that:
/// - starts with the user's prompt (and optional previous conversation)
/// - repeatedly calls the model
/// - executes any requested tools (except Write/Bash in Ask mode)
/// - feeds tool results back to the model
/// - stops when the model responds without tool calls
/// If `confirm_destructive` is Some (CLI mode), destructive commands use the callback.
/// If None (TUI mode), returns `NeedsConfirmation` so the TUI can show a popup.
/// If `on_progress` is Some, it is called with verbose updates during processing.
pub async fn chat(
    config: &Config,
    prompt: &str,
    mode: &str,
    confirm_destructive: Option<ConfirmDestructive>,
    previous_messages: Option<Vec<Value>>,
    on_progress: Option<OnProgress>,
) -> Result<ChatResult, ChatError> {
    let client = Client::with_config(config.openai_config.clone());

    let mut messages: Vec<Value> = previous_messages.unwrap_or_default();
    messages.push(json!({
        "role": "user",
        "content": prompt,
    }));
    let mut messages = Arc::new(messages);
    let mut tool_log = Arc::new(Vec::<String>::new());

    let tools_defs = tools::definitions();
    let tools_list = tools::all();

    run_agent_loop(
        &client,
        config,
        &tools_defs,
        &tools_list,
        &mut messages,
        &mut tool_log,
        mode,
        &confirm_destructive,
        on_progress.as_deref(),
    )
    .await
}

async fn run_agent_loop(
    client: &Client<OpenAIConfig>,
    config: &Config,
    tools_defs: &[Value],
    tools_list: &[Box<dyn tools::Tool>],
    messages: &mut Arc<Vec<Value>>,
    tool_log: &mut Arc<Vec<String>>,
    mode: &str,
    confirm_destructive: &Option<ConfirmDestructive>,
    on_progress: Option<&(dyn Fn(&str) + Send)>,
) -> Result<ChatResult, ChatError> {
    loop {
        if let Some(ref progress) = on_progress {
            progress("Calling API...");
        }
        let response: Value = client
            .chat()
            .create_byot(json!({
                "model": config.model_id,
                "messages": messages.as_ref(),
                "tool_choice": "auto",
                "tools": tools_defs,
            }))
            .await
            .map_err(|e| {
                let s = e.to_string();
                if s.contains("401") && s.contains("cookie auth") {
                    return ChatError::ApiAuth(
                        "API error (401): No cookie auth credentials found. Check OPENROUTER_API_KEY in .env (see env.example).".to_string(),
                    );
                }
                if s.contains("\"error\"") {
                    if let Some((_, rest)) = s.split_once("\"message\":\"") {
                        if let Some((msg, _)) = rest.split_once('"') {
                            return ChatError::ApiMessage(msg.to_string());
                        }
                    }
                }
                ChatError::Other(e.into())
            })?;

        let assistant_message = &response["choices"][0]["message"];
        Arc::make_mut(messages).push(assistant_message.clone());

        let tool_calls_opt = assistant_message
            .get("tool_calls")
            .and_then(|v| v.as_array());

        let Some(tool_calls) = tool_calls_opt else {
            let content = assistant_message["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            return Ok(make_complete(&content, tool_log.as_ref(), messages.as_ref()));
        };

        if tool_calls.is_empty() {
            let content = assistant_message["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            return Ok(make_complete(&content, tool_log.as_ref(), messages.as_ref()));
        }

        for tool_call in tool_calls {
            let id = tool_call["id"].as_str().unwrap_or_default().to_string();
            let function = &tool_call["function"];
            let name = function["name"].as_str().unwrap_or_default();
            let args_str = function["arguments"].as_str().unwrap_or("{}");

            let args: Value = serde_json::from_str(args_str)
                .map_err(|e| ChatError::ToolArgs {
                    tool: name.to_string(),
                    source: e,
                })?;

            let args_preview = tools_list
                .iter()
                .find(|t| t.name() == name)
                .map(|t| t.args_preview(&args))
                .unwrap_or_default();
            let log_line = format!("→ {}: {}", name, args_preview);
            Arc::make_mut(tool_log).push(log_line.clone());
            if let Some(ref progress) = on_progress {
                progress(&log_line);
            }

            let ask_mode_disabled = "Ask mode: file creation/modification and command execution are disabled. Use only the Read tool to read files, then respond with an explanation.";

            const WRITE_NAME: &str = "Write";
            const BASH_NAME: &str = "Bash";

            let result = if is_ask_mode(mode)
                && (name == WRITE_NAME || name == BASH_NAME)
            {
                ask_mode_disabled.to_string()
            } else {
                match tools_list.iter().find(|t| t.name() == name) {
                    Some(tool) => {
                        if name == BASH_NAME {
                            if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                                if tools::is_destructive(command) {
                                    if let Some(cb) = confirm_destructive {
                                        let confirmed = cb(command);
                                        if !confirmed {
                                            "Command cancelled (destructive command not confirmed)."
                                                .to_string()
                                        } else {
                                            tool.execute(&args)
                                                .unwrap_or_else(|e| format!("Error: {}", e))
                                        }
                                    } else {
                                        return Ok(ChatResult::NeedsConfirmation {
                                            command: command.to_string(),
                                            state: ConfirmState {
                                                messages: Arc::clone(messages),
                                                tool_log: Arc::clone(tool_log),
                                                tool_call_id: id.clone(),
                                                mode: mode.to_string(),
                                                tools: tools_defs.to_vec(),
                                                command: command.to_string(),
                                            },
                                        });
                                    }
                                } else {
                                    tool.execute(&args)
                                        .unwrap_or_else(|e| format!("Error: {}", e))
                                }
                            } else {
                                "Error: missing command argument".to_string()
                            }
                        } else {
                            tool.execute(&args)
                                .unwrap_or_else(|e| format!("Error: {}", e))
                        }
                    }
                    None => format!("Error: unknown tool '{}'", name),
                }
            };

            Arc::make_mut(messages).push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": result,
            }));
        }
    }
}

/// Resume the chat loop after user confirmed or cancelled a destructive command.
/// Call this when you receive `NeedsConfirmation` and the user has answered.
pub async fn chat_resume(
    config: &Config,
    state: ConfirmState,
    confirmed: bool,
) -> Result<ChatResult, ChatError> {
    let client = Client::with_config(config.openai_config.clone());

    let bash_tool = tools::BashTool;
    let result = if confirmed {
        bash_tool.execute(&json!({ "command": state.command }))
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

    run_agent_loop(
        &client,
        config,
        &tools_defs,
        &tools_list,
        &mut messages,
        &mut tool_log,
        &state.mode,
        &None, // No callback on resume; if another destructive command, return NeedsConfirmation again
        None,  // No progress callback on resume
    )
    .await
}
