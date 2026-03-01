//! Chat and API error types.

/// Errors from the chat/agent pipeline.
#[derive(Debug)]
pub enum ChatError {
    ApiAuth(String),
    ApiMessage(String),
    /// Rate-limited or temporarily overloaded (retryable).
    RateLimited(String),
    ToolArgs {
        tool: String,
        source: serde_json::Error,
    },
    /// The request was cancelled by the user.
    Cancelled,
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl ChatError {
    /// Whether this error is transient and the request should be retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            ChatError::RateLimited(_) => true,
            ChatError::ApiMessage(msg) => {
                let lower = msg.to_lowercase();
                lower.contains("timeout")
                    || lower.contains("overloaded")
                    || lower.contains("rate limit")
                    || lower.contains("service unavailable")
                    || lower.contains("bad gateway")
                    || lower.contains("too many requests")
            }
            ChatError::Other(e) => {
                let lower = e.to_string().to_lowercase();
                lower.contains("connection") || lower.contains("timeout")
            }
            ChatError::ApiAuth(_) | ChatError::Cancelled | ChatError::ToolArgs { .. } => false,
        }
    }
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatError::ApiAuth(msg) => write!(f, "{}", msg),
            ChatError::ApiMessage(msg) => write!(f, "API error: {}", msg),
            ChatError::RateLimited(msg) => write!(f, "Rate limited: {}", msg),
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
            ChatError::Cancelled
            | ChatError::ApiAuth(_)
            | ChatError::ApiMessage(_)
            | ChatError::RateLimited(_) => None,
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
    // Detect rate limiting / overloaded responses.
    if s.contains("429") || s.contains("rate limit") || s.contains("Rate limit") {
        return ChatError::RateLimited(s);
    }
    if (s.contains("503") || s.contains("502")) && s.contains("overloaded") {
        return ChatError::RateLimited(s);
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s)
        && let Some(msg) = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
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
        let e = std::io::Error::other(r#"{"error":{"message":"Invalid model specified"}}"#);
        let err = map_api_error(e);
        match &err {
            ChatError::ApiMessage(msg) => assert_eq!(msg, "Invalid model specified"),
            _ => panic!("expected ApiMessage, got {:?}", err),
        }
    }

    #[test]
    fn map_api_error_json_rate_limit_message() {
        let e = std::io::Error::other(r#"{"error":{"message":"Rate limit exceeded"}}"#);
        let err = map_api_error(e);
        assert!(matches!(&err, ChatError::RateLimited(_)));
        assert!(err.is_retryable());
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

    #[test]
    fn map_api_error_rate_limited_429() {
        let e = std::io::Error::other("HTTP 429 Too Many Requests - rate limit exceeded");
        let err = map_api_error(e);
        assert!(matches!(&err, ChatError::RateLimited(_)));
        assert!(err.is_retryable());
    }

    #[test]
    fn map_api_error_overloaded_503() {
        let e = std::io::Error::other("503 Service Unavailable - model overloaded");
        let err = map_api_error(e);
        assert!(matches!(&err, ChatError::RateLimited(_)));
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_api_message_timeout() {
        let err = ChatError::ApiMessage("Request timeout".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_api_message_overloaded() {
        let err = ChatError::ApiMessage("Model is overloaded, please retry".to_string());
        assert!(err.is_retryable());
    }

    #[test]
    fn is_retryable_other_connection() {
        let e = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
        let err = ChatError::Other(e.into());
        assert!(err.is_retryable());
    }

    #[test]
    fn is_not_retryable_auth() {
        let err = ChatError::ApiAuth("Invalid API key".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn is_not_retryable_cancelled() {
        assert!(!ChatError::Cancelled.is_retryable());
    }

    #[test]
    fn is_not_retryable_regular_api_message() {
        let err = ChatError::ApiMessage("Invalid model ID".to_string());
        assert!(!err.is_retryable());
    }
}
