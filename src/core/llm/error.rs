//! Chat and API error types.

/// Errors from the chat/agent pipeline.
#[derive(Debug)]
pub enum ChatError {
    ApiAuth(String),
    ApiMessage(String),
    ToolArgs {
        tool: String,
        source: serde_json::Error,
    },
    /// The request was cancelled by the user.
    Cancelled,
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
            ChatError::Cancelled => write!(f, "Request cancelled"),
            ChatError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ChatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ChatError::ToolArgs { source, .. } => Some(source),
            ChatError::Other(e) => e.source(),
            ChatError::Cancelled | ChatError::ApiAuth(_) | ChatError::ApiMessage(_) => None,
        }
    }
}

/// Map async-openai or API errors into ChatError.
pub fn map_api_error<E>(e: E) -> ChatError
where
    E: std::fmt::Display + Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    let s = e.to_string();
    if s.contains("401") && s.contains("cookie auth") {
        return ChatError::ApiAuth(
            "API error (401): No cookie auth credentials found. Check OPENROUTER_API_KEY in .env (see env.example).".to_string(),
        );
    }
    if s.contains("\"error\"")
        && let Some((_, rest)) = s.split_once("\"message\":\"")
        && let Some((msg, _)) = rest.split_once('"')
    {
        return ChatError::ApiMessage(msg.to_string());
    }
    ChatError::Other(e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_api_error_401_cookie_auth() {
        let e = std::io::Error::other("401 and cookie auth");
        let err = map_api_error(e);
        match &err {
            ChatError::ApiAuth(msg) => {
                assert!(msg.contains("OPENROUTER_API_KEY"));
            }
            _ => panic!("expected ApiAuth, got {:?}", err),
        }
    }

    #[test]
    fn map_api_error_json_message() {
        let e = std::io::Error::other(r#"{"error":{"message":"Rate limit exceeded"}}"#);
        let err = map_api_error(e);
        match &err {
            ChatError::ApiMessage(msg) => assert_eq!(msg, "Rate limit exceeded"),
            _ => panic!("expected ApiMessage, got {:?}", err),
        }
    }

    #[test]
    fn map_api_error_generic() {
        let e = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let err = map_api_error(e);
        match &err {
            ChatError::Other(_) => {}
            _ => panic!("expected Other, got {:?}", err),
        }
    }
}
