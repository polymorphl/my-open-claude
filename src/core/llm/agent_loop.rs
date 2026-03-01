//! Agent loop: streaming, tool execution, repeat until done.

use async_openai::Client;
use async_openai::config::OpenAIConfig;
use futures::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::core::confirm::ConfirmDestructive;
use crate::core::tools;

use super::context;
use super::stream::{MAX_CONTENT_BYTES, TokenUsage, merge_tool_call_delta, parse_usage};
use super::tool_execution;
use super::undo;
use super::{ChatError, ChatResult, map_api_error};

/// Maximum number of retries for transient API errors.
const MAX_RETRIES: u32 = 3;
/// Base delay in milliseconds for exponential backoff (1s, 2s, 4s).
const BASE_DELAY_MS: u64 = 1000;

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

/// Callbacks and options for the agent loop (confirmation, progress, streaming, cancellation).
pub(super) struct AgentLoopCallbacks<'a> {
    pub confirm_destructive: &'a Option<ConfirmDestructive>,
    pub on_progress: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub on_content_chunk: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub cancel_token: Option<&'a CancellationToken>,
}

/// Core parameters for the agent loop (API, model, tools, messages).
pub(super) struct AgentLoopParams<'a> {
    pub client: &'a Client<OpenAIConfig>,
    pub model: &'a str,
    pub context_length: u64,
    pub tools_defs: &'a [Value],
    pub tools_list: &'a [Box<dyn tools::Tool>],
    pub messages: &'a mut Arc<Vec<Value>>,
    pub tool_log: &'a mut Arc<Vec<String>>,
    pub mode: &'a str,
    pub undo_stack: Option<undo::SharedUndoStack>,
}

/// Result of a single streaming API call: content, tool calls, and token usage.
struct StreamResult {
    content: String,
    tool_calls: Vec<Value>,
    usage: TokenUsage,
}

/// Make a single streaming API call and collect the full response.
async fn stream_api_call(
    client: &Client<OpenAIConfig>,
    model: &str,
    messages: &[Value],
    tools_defs: &[Value],
    cancel_token: Option<&CancellationToken>,
    on_content_chunk: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<StreamResult, ChatError> {
    let chat_api = client.chat();
    let stream_future = chat_api.create_stream_byot::<_, Value>(json!({
        "model": model,
        "messages": messages,
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
    let mut last_usage = TokenUsage::default();

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
                if let Some(cb) = on_content_chunk {
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

    Ok(StreamResult {
        content: full_content,
        tool_calls: accumulated_tool_calls,
        usage: last_usage,
    })
}

/// Make a streaming API call with retry on transient errors (rate limit, timeout, overload).
async fn stream_api_call_with_retry(
    client: &Client<OpenAIConfig>,
    model: &str,
    messages: &[Value],
    tools_defs: &[Value],
    cancel_token: Option<&CancellationToken>,
    on_progress: Option<&(dyn Fn(&str) + Send + Sync)>,
    on_content_chunk: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<StreamResult, ChatError> {
    for attempt in 0..=MAX_RETRIES {
        match stream_api_call(
            client,
            model,
            messages,
            tools_defs,
            cancel_token,
            on_content_chunk,
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempt < MAX_RETRIES => {
                let delay_ms = BASE_DELAY_MS * 2u64.pow(attempt);
                if let Some(progress) = on_progress {
                    progress(&format!(
                        "Retrying in {}s... (attempt {}/{})",
                        delay_ms / 1000,
                        attempt + 1,
                        MAX_RETRIES
                    ));
                }
                // Sleep while respecting cancellation.
                let sleep = tokio::time::sleep(Duration::from_millis(delay_ms));
                if let Some(token) = cancel_token {
                    tokio::select! {
                        biased;
                        _ = token.cancelled() => return Err(ChatError::Cancelled),
                        _ = sleep => {}
                    }
                } else {
                    sleep.await;
                }
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("loop runs MAX_RETRIES+1 times")
}

/// Run the agent loop: call API, stream response, execute tools, repeat.
pub(super) async fn run_agent_loop(
    params: AgentLoopParams<'_>,
    callbacks: AgentLoopCallbacks<'_>,
) -> Result<ChatResult, ChatError> {
    let cancel_token = callbacks.cancel_token;
    let mut init_file_written = false;

    loop {
        // Check cancellation before starting a new API call.
        if cancel_token.is_some_and(|t| t.is_cancelled()) {
            return Err(ChatError::Cancelled);
        }

        // Truncate context if it exceeds the model's window.
        context::truncate_if_needed(Arc::make_mut(params.messages), params.context_length);

        if let Some(ref progress) = callbacks.on_progress {
            progress("Calling API...");
        }

        let result = stream_api_call_with_retry(
            params.client,
            params.model,
            params.messages.as_ref(),
            params.tools_defs,
            cancel_token,
            callbacks.on_progress,
            callbacks.on_content_chunk,
        )
        .await?;

        let last_usage = result.usage;

        let assistant_message = if !result.tool_calls.is_empty() {
            json!({
                "role": "assistant",
                "content": result.content,
                "tool_calls": result.tool_calls.iter().map(|tc| json!({
                    "id": tc["id"].as_str().unwrap_or(""),
                    "type": "function",
                    "function": tc["function"].clone()
                })).collect::<Vec<_>>()
            })
        } else {
            json!({
                "role": "assistant",
                "content": result.content
            })
        };

        Arc::make_mut(params.messages).push(assistant_message);

        // Summarize Write/Edit tool arguments to reduce context bloat on subsequent turns.
        context::summarize_write_args_in_last(Arc::make_mut(params.messages).as_mut_slice());

        // Extract tool_calls from the (potentially summarized) message.
        let tool_calls_vec = match params
            .messages
            .last()
            .and_then(|m| m.get("tool_calls"))
            .and_then(|v| v.as_array())
        {
            Some(tc) if !tc.is_empty() => tc.to_vec(),
            _ => {
                return Ok(make_complete(
                    &result.content,
                    params.tool_log.as_ref(),
                    params.messages.as_ref(),
                    last_usage,
                ));
            }
        };
        let tool_calls = &tool_calls_vec;

        // Check cancellation before executing tools.
        if cancel_token.is_some_and(|t| t.is_cancelled()) {
            return Err(ChatError::Cancelled);
        }

        // Create an undo batch for this iteration (captures file state before modifications).
        let mut undo_batch = undo::UndoBatch::default();

        // Check if all tool calls in this batch are read-only (safe to parallelize).
        let all_read_only = tool_calls.iter().all(|tc| {
            let name = tc["function"]["name"].as_str().unwrap_or_default();
            params
                .tools_list
                .iter()
                .find(|t| t.name() == name)
                .is_some_and(|t| t.is_read_only())
        });

        if all_read_only && tool_calls.len() > 1 {
            // Execute read-only tools in parallel using blocking tasks.
            // No undo needed for read-only tools.
            let mode = params.mode.to_string();
            let tools_list = params.tools_list;
            let tool_calls_owned: Vec<Value> = tool_calls.to_vec();

            let handles: Vec<_> = tool_calls_owned
                .into_iter()
                .map(|tc| {
                    let mode = mode.clone();
                    let tools_ref: *const [Box<dyn tools::Tool>] = tools_list;
                    // SAFETY: tools_list is borrowed from params which outlives this scope.
                    // We join all tasks before the end of this block via join_all.await below.
                    let tools_static: &'static [Box<dyn tools::Tool>] = unsafe { &*tools_ref };
                    tokio::task::spawn_blocking(move || {
                        tool_execution::execute_read_only_tool_call(&tc, tools_static, &mode)
                    })
                })
                .collect();

            let results = futures::future::join_all(handles).await;

            for join_result in results {
                let tool_result = join_result.map_err(|e| ChatError::Other(e.into()))??;

                Arc::make_mut(params.tool_log).push(tool_result.log_line.clone());
                if let Some(ref progress) = callbacks.on_progress {
                    progress(&tool_result.log_line);
                }
                Arc::make_mut(params.messages).push(json!({
                    "role": "tool",
                    "tool_call_id": tool_result.tool_call_id,
                    "content": tool_result.content,
                }));
            }
        } else {
            // Sequential execution for write tools, single tool calls, or mixed batches.
            for tool_call in tool_calls {
                if cancel_token.is_some_and(|t| t.is_cancelled()) {
                    return Err(ChatError::Cancelled);
                }

                let mut tool_ctx = tool_execution::ToolCallContext {
                    confirm_destructive: callbacks.confirm_destructive,
                    tools_defs: params.tools_defs,
                    messages: params.messages,
                    tool_log: params.tool_log,
                    on_progress: callbacks.on_progress,
                    init_file_written: Some(&mut init_file_written),
                    undo_batch: Some(&mut undo_batch),
                    undo_stack: params.undo_stack.clone(),
                };
                if let Some(needs_confirmation) = tool_execution::execute_tool_call(
                    tool_call,
                    params.tools_list,
                    params.mode,
                    &mut tool_ctx,
                )? {
                    return Ok(needs_confirmation);
                }
            }
        }

        // Push the undo batch to the shared stack (only if files were captured).
        if !undo_batch.is_empty()
            && let Some(ref stack) = params.undo_stack
        {
            match stack.lock() {
                Ok(mut s) => s.push_batch(undo_batch),
                Err(e) => {
                    // Poisoned mutex: recover guard and still push to avoid losing undo data.
                    log::warn!("Undo stack mutex poisoned; recovering: {}", e);
                    e.into_inner().push_batch(undo_batch);
                }
            }
        }
    }
}
