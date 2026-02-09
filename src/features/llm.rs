use async_openai::Client;
use serde_json::{json, Value};

use crate::features::tools;

/// Run an agent loop that:
/// - starts with the user's prompt
/// - repeatedly calls the model
/// - executes any requested tools
/// - feeds tool results back to the model
/// - stops when the model responds without tool calls
/// and finally returns the assistant's message content.
pub async fn chat(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = crate::features::config::load();
    let client = Client::with_config(config);

    // Initial conversation with the user's prompt.
    let mut messages = vec![json!({
        "role": "user",
        "content": prompt,
    })];

    let tools = tools::definitions();

    loop {
        let response: Value = client
            .chat()
            .create_byot(json!({
                "model": "anthropic/claude-haiku-4.5",
                "messages": messages,
                "tool_choice": "auto",
                "tools": tools,
            }))
            .await?;

        let assistant_message = &response["choices"][0]["message"];

        // Record the assistant's response in the conversation history.
        messages.push(assistant_message.clone());

        // Check if the assistant requested any tools.
        let tool_calls_opt = assistant_message.get("tool_calls").and_then(|v| v.as_array());

        // If there are no tool calls, we're done – return the final content.
        let Some(tool_calls) = tool_calls_opt else {
            let content = assistant_message["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            return Ok(content);
        };

        // If tool_calls exists but is empty, also treat this as completion.
        if tool_calls.is_empty() {
            let content = assistant_message["content"]
                .as_str()
                .unwrap_or("")
                .to_string();
            return Ok(content);
        }

        // Execute each requested tool and append its result to the messages.
        for tool_call in tool_calls {
            let id = tool_call["id"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let function = &tool_call["function"];
            let name = function["name"].as_str().unwrap_or_default();
            let args_str = function["arguments"].as_str().unwrap_or("{}");

            // Parse the arguments JSON string.
            let args: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({}));

            // Execute the requested tool.
            let result = if name == "Read" {
                if let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) {
                    tools::read::execute(file_path)
                } else {
                    "Error: missing file_path argument".to_string()
                }
            } else if name == "Write" {
                match (
                    args.get("file_path").and_then(|v| v.as_str()),
                    args.get("content").and_then(|v| v.as_str()),
                ) {
                    (Some(file_path), Some(content)) => tools::write::execute(file_path, content),
                    _ => "Error: missing file_path or content argument".to_string(),
                }
            } else {
                // Unknown tool – return an error message to the model.
                format!("Error: unknown tool '{}'", name)
            };

            // Add the tool result to the conversation history.
            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": result,
            }));
        }
        // Loop continues: the next iteration sends updated `messages` back to the model.
    }
}
