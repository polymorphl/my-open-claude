//! Agent loop: streaming, tool execution, repeat until done.

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use futures::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::core::config::Config;
use crate::core::confirm::ConfirmDestructive;
use crate::core::tools;

use super::context;
use super::stream::{MAX_CONTENT_BYTES, TokenUsage, merge_tool_call_delta, parse_usage};
use super::tool_execution;
use super::{ChatError, ChatResult, map_api_error};

fn make_complete(
    content: &str,
    tool_log: &[String],
    messages: &[Value],
    usage: TokenUsage,
) -> ChatResult {
    ChatResult::Complete {
        content: content.to_string(),
        tool_log: tool_log.to_vec(),
        messages: messages.to_vec(),
        usage,
    }
}

/// Run the agent loop: call API, stream response, execute tools, repeat.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_agent_loop(
    client: &Client<OpenAIConfig>,
    _config: &Config,
    model: &str,
    context_length: u64,
    tools_defs: &[Value],
    tools_list: &[Box<dyn tools::Tool>],
    messages: &mut Arc<Vec<Value>>,
    tool_log: &mut Arc<Vec<String>>,
    mode: &str,
    confirm_destructive: &Option<ConfirmDestructive>,
    on_progress: Option<&(dyn Fn(&str) + Send)>,
    on_content_chunk: Option<&(dyn Fn(&str) + Send)>,
    cancel_token: Option<&CancellationToken>,
) -> Result<ChatResult, ChatError> {
    let mut last_usage = TokenUsage::default();

    loop {
        // Check cancellation before starting a new API call.
        if cancel_token.is_some_and(|t| t.is_cancelled()) {
            return Err(ChatError::Cancelled);
        }

        // Truncate context if it exceeds the model's window.
        context::truncate_if_needed(Arc::make_mut(messages), context_length);

        if let Some(ref progress) = on_progress {
            progress("Calling API...");
        }

        // Start the streaming API call, racing against cancellation.
        let chat_api = client.chat();
        let stream_future = chat_api.create_stream_byot::<_, Value>(json!({
            "model": model,
            "messages": messages.as_ref(),
            "tool_choice": "auto",
            "tools": tools_defs,
            "stream": true,
        }));

        let stream_result = if let Some(token) = cancel_token {
            tokio::select! {
                biased;
                _ = token.cancelled() => {
                    return Err(ChatError::Cancelled);
                }
                result = stream_future => result,
            }
        } else {
            stream_future.await
        };

        let mut stream = stream_result.map_err(map_api_error)?;

        let mut full_content = String::new();
        let mut accumulated_tool_calls: Vec<Value> = Vec::new();

        // Read stream chunks, racing against cancellation.
        loop {
            let chunk_opt = if let Some(token) = cancel_token {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => {
                        return Err(ChatError::Cancelled);
                    }
                    chunk = stream.next() => chunk,
                }
            } else {
                stream.next().await
            };

            let Some(chunk_result) = chunk_opt else { break };
            let chunk = chunk_result.map_err(map_api_error)?;

            if let Some(err) = chunk.get("error") {
                let msg = err
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                return Err(ChatError::ApiMessage(msg.to_string()));
            }

            // Capture token usage from the final chunk (OpenRouter includes it).
            if let Some(usage) = parse_usage(&chunk) {
                last_usage = usage;
            }

            let choices = chunk.get("choices").and_then(|c| c.as_array());
            let Some(choices) = choices else { continue };
            let Some(choice) = choices.first() else {
                continue;
            };
            let delta = &choice["delta"];

            if let Some(content) = delta["content"].as_str() {
                if !content.is_empty() && full_content.len() + content.len() <= MAX_CONTENT_BYTES {
                    full_content.push_str(content);
                    if let Some(ref cb) = on_content_chunk {
                        cb(content);
                    }
                } else if full_content.len() >= MAX_CONTENT_BYTES {
                    break;
                }
            }

            if let Some(tc_arr) = delta["tool_calls"].as_array() {
                for tc in tc_arr {
                    merge_tool_call_delta(&mut accumulated_tool_calls, tc);
                }
            }
        }

        let tool_calls_opt = if accumulated_tool_calls.is_empty() {
            None
        } else {
            Some(accumulated_tool_calls)
        };

        let assistant_message = if let Some(ref tcs) = tool_calls_opt {
            json!({
                "role": "assistant",
                "content": full_content,
                "tool_calls": tcs.iter().map(|tc| json!({
                    "id": tc["id"].as_str().unwrap_or(""),
                    "type": "function",
                    "function": tc["function"].clone()
                })).collect::<Vec<_>>()
            })
        } else {
            json!({
                "role": "assistant",
                "content": full_content
            })
        };

        Arc::make_mut(messages).push(assistant_message.clone());

        // Summarize Write/Edit tool arguments to reduce context bloat on subsequent turns.
        context::summarize_write_args_in_last(Arc::make_mut(messages).as_mut_slice());

        let tool_calls_opt = assistant_message
            .get("tool_calls")
            .and_then(|v| v.as_array());

        let Some(tool_calls) = tool_calls_opt else {
            return Ok(make_complete(
                &full_content,
                tool_log.as_ref(),
                messages.as_ref(),
                last_usage.clone(),
            ));
        };

        if tool_calls.is_empty() {
            return Ok(make_complete(
                &full_content,
                tool_log.as_ref(),
                messages.as_ref(),
                last_usage.clone(),
            ));
        }

        // Check cancellation before executing tools.
        if cancel_token.is_some_and(|t| t.is_cancelled()) {
            return Err(ChatError::Cancelled);
        }

        for tool_call in tool_calls {
            // Check cancellation before each tool call.
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                return Err(ChatError::Cancelled);
            }

            if let Some(needs_confirmation) = tool_execution::execute_tool_call(
                tool_call,
                tools_list,
                mode,
                confirm_destructive,
                tools_defs,
                messages,
                tool_log,
                on_progress,
            )? {
                return Ok(needs_confirmation);
            }
        }
    }
}
