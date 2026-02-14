# Architecture & Flows

## Agent loop

The core chat flow: the model is called, streams a response, and may request tool execution. The loop repeats until the model responds without tool calls, or returns `NeedsConfirmation` for a destructive command (e.g. `rm`, `rmdir`).

```mermaid
flowchart TD
    Start[chat] --> Truncate[Truncate context if needed]
    Truncate --> APICall[Stream API call]
    APICall --> Stream[Read chunks]
    Stream --> ToolCalls{Tool calls?}
    ToolCalls -->|No| Complete[Return Complete]
    ToolCalls -->|Yes| Execute[Execute each tool]
    Execute --> Destructive{Destructive and needs confirm?}
    Destructive -->|Yes| NeedsConf[Return NeedsConfirmation]
    Destructive -->|No| Append[Append result to messages]
    Append --> Truncate
```

## Entry point & modes

The application supports two modes: single-prompt (one request then exit) and TUI (interactive chat). Both use the same `core` modules.

```mermaid
flowchart TB
    subgraph Entry [Entry point]
        Main[main.rs]
    end
    Main --> Config[config load]
    Main --> Workspace[workspace detect]
    Config --> Branch{Has -p flag?}
    Branch -->|Yes| DirectChat[chat directly]
    Branch -->|No| TUIRun[tui run]
    DirectChat --> LLM[core/llm]
    TUIRun --> TUI[tui]
    TUI --> LLM
    LLM --> Tools[core/tools]
```
