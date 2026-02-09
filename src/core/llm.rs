use async_openai::config::OpenAIConfig;
use async_openai::Client;
use serde_json::{Value, json};

use crate::confirm::ConfirmDestructive;
use crate::core::tools;

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
    pub(super) messages: Vec<Value>,
    pub(super) tool_log: Vec<String>,
    pub(super) tool_call_id: String,
    pub(super) mode: String,
    pub(super) tools: Vec<Value>,
    pub(super) command: String,
}

/// Run an agent loop that:
/// - starts with the user's prompt (and optional previous conversation)
/// - repeatedly calls the model
/// - executes any requested tools (except Write/Bash in Ask mode)
/// - feeds tool results back to the model
/// - stops when the model responds without tool calls
/// If `confirm_destructive` is Some (CLI mode), destructive commands use the callback.
/// If None (TUI mode), returns `NeedsConfirmation` so the TUI can show a popup.
pub async fn chat(
    prompt: &str,
    mode: &str,
    confirm_destructive: Option<ConfirmDestructive>,
    previous_messages: Option<Vec<Value>>,
) -> Result<ChatResult, Box<dyn std::error::Error>> {
    let config = crate::core::config::load();
    let client = Client::with_config(config);

    // Start from previous conversation (if any) and append the new user message.
    let mut messages = previous_messages.unwrap_or_default();
    messages.push(json!({
        "role": "user",
        "content": prompt,
    }));

    let tools = tools::definitions();
    let mut tool_log: Vec<String> = Vec::new();

    run_agent_loop(&client, &tools, &mut messages, &mut tool_log, mode, &confirm_destructive).await
}

async fn run_agent_loop(
    client: &Client<OpenAIConfig>,
    tools: &[Value],
    messages: &mut Vec<Value>,
    tool_log: &mut Vec<String>,
    mode: &str,
    confirm_destructive: &Option<ConfirmDestructive>,
) -> Result<ChatResult, Box<dyn std::error::Error>> {
    let model_id = crate::core::config::model();
    loop {
        let response: Value = client
            .chat()
            .create_byot(json!({
                "model": model_id.clone(),
                "messages": messages,
                "tool_choice": "auto",
                "tools": tools,
            }))
            .await
            .map_err(|e| {
                let s = e.to_string();
                // When the API returns 401/403 etc., the body is in the error string; show a clearer message
                if s.contains("401") && s.contains("cookie auth") {
                    return Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "API error (401): No cookie auth credentials found. Check OPENROUTER_API_KEY in .env (see env.example).",
                    )) as Box<dyn std::error::Error>;
                }
                if s.contains("\"error\"") {
                    if let Some((_, rest)) = s.split_once("\"message\":\"") {
                        if let Some((msg, _)) = rest.split_once('"') {
                            return Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!("API error: {}", msg),
                            )) as Box<dyn std::error::Error>;
                        }
                    }
                }
                e.into()
            })?;

        let assistant_message = &response["choices"][0]["message"];

        // Record the assistant's response in the conversation history.
        messages.push(assistant_message.clone());

        // Check if the assistant requested any tools.
        let tool_calls_opt = assistant_message
            .get("tool_calls")
            .and_then(|v| v.as_array());

        // If there are no tool calls, we're done – return the final content and full history.
        let Some(tool_calls) = tool_calls_opt else {
            let content = assistant_message["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            return Ok(ChatResult::Complete {
                content,
                tool_log: tool_log.clone(),
                messages: messages.clone(),
            });
        };

        // If tool_calls exists but is empty, also treat this as completion.
        if tool_calls.is_empty() {
            let content = assistant_message["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            return Ok(ChatResult::Complete {
                content,
                tool_log: tool_log.clone(),
                messages: messages.clone(),
            });
        }

        // Execute each requested tool and append its result to the messages.
        for tool_call in tool_calls {
            let id = tool_call["id"].as_str().unwrap_or_default().to_string();
            let function = &tool_call["function"];
            let name = function["name"].as_str().unwrap_or_default();
            let args_str = function["arguments"].as_str().unwrap_or("{}");

            // Parse the arguments JSON string.
            let args: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));

            // Log for verbose display (before execution).
            let args_preview = match name {
                n if n == tools::bash::NAME => args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                n if n == tools::read::NAME || n == tools::write::NAME => args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                _ => String::new(),
            };
            tool_log.push(format!("→ {}: {}", name, args_preview));

            // In Ask mode: allow Read (for explaining), refuse only Write and Bash.
            let result = if is_ask_mode(mode) && (name == tools::write::NAME || name == tools::bash::NAME) {
                "Mode Ask : création/modification de fichiers et exécution de commandes désactivées. Utilisez uniquement l'outil Read pour lire des fichiers, puis répondez par une explication."
                    .to_string()
            } else {
                match name {
                    n if n == tools::bash::NAME => {
                        if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                            if tools::bash::is_destructive(command) {
                                if let Some(cb) = confirm_destructive {
                                    let confirmed = cb(command);
                                    if !confirmed {
                                        "Command cancelled (destructive command not confirmed)."
                                            .to_string()
                                    } else {
                                        tools::bash::execute(command)
                                    }
                                } else {
                                    // TUI mode: return NeedsConfirmation so the TUI can show a popup
                                    return Ok(ChatResult::NeedsConfirmation {
                                        command: command.to_string(),
                                        state: ConfirmState {
                                            messages: messages.clone(),
                                            tool_log: tool_log.clone(),
                                            tool_call_id: id.clone(),
                                            mode: mode.to_string(),
                                            tools: tools.to_vec(),
                                            command: command.to_string(),
                                        },
                                    });
                                }
                            } else {
                                tools::bash::execute(command)
                            }
                        } else {
                            "Error: missing command argument".to_string()
                        }
                    }
                    n if n == tools::read::NAME => {
                        if let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) {
                            tools::read::execute(file_path)
                        } else {
                            "Error: missing file_path argument".to_string()
                        }
                    }
                    n if n == tools::write::NAME => {
                        match (
                            args.get("file_path").and_then(|v| v.as_str()),
                            args.get("content").and_then(|v| v.as_str()),
                        ) {
                            (Some(file_path), Some(content)) => {
                                tools::write::execute(file_path, content)
                            }
                            _ => "Error: missing file_path or content argument".to_string(),
                        }
                    }
                    _ => format!("Error: unknown tool '{}'", name),
                }
            };

            // Add the tool result to the conversation history.
            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": result,
            }));
        }
        // Loop continues: next iteration sends updated `messages` back to the model.
    }
}

/// Resume the chat loop after user confirmed or cancelled a destructive command.
/// Call this when you receive `NeedsConfirmation` and the user has answered.
pub async fn chat_resume(
    state: ConfirmState,
    confirmed: bool,
) -> Result<ChatResult, Box<dyn std::error::Error>> {
    let config = crate::core::config::load();
    let client = Client::with_config(config);

    let result = if confirmed {
        tools::bash::execute(&state.command)
    } else {
        "Command cancelled (destructive command not confirmed).".to_string()
    };

    let mut messages = state.messages;
    messages.push(json!({
        "role": "tool",
        "tool_call_id": state.tool_call_id,
        "content": result,
    }));

    let mut tool_log = state.tool_log;
    run_agent_loop(
        &client,
        &state.tools,
        &mut messages,
        &mut tool_log,
        &state.mode,
        &None, // No callback on resume; if another destructive command, return NeedsConfirmation again
    )
    .await
}
