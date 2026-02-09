[![progress-banner](https://backend.codecrafters.io/progress/claude-code/2e2ff4a4-fa68-4b26-9b97-799a4ef9e337)](https://app.codecrafters.io/users/codecrafters-bot?r=2qF)

This is a starting point for Rust solutions to the
["Build Your own Claude Code" Challenge](https://codecrafters.io/challenges/claude-code).

Claude Code is an AI coding assistant that uses Large Language Models (LLMs) to
understand code and perform actions through tool calls. In this challenge,
you'll build your own Claude Code from scratch by implementing an LLM-powered
coding assistant.

Along the way you'll learn about HTTP RESTful APIs, OpenAI-compatible tool
calling, agent loop, and how to integrate multiple tools into an AI assistant.

**Note**: If you're viewing this repo on GitHub, head over to
[codecrafters.io](https://codecrafters.io) to try the challenge.

# Passing the first stage

The entry point for your `claude-code` implementation is in `src/main.rs`. Study
and uncomment the relevant code, and submit to pass the first stage:

```sh
codecrafters submit
```

# Configuration

The application uses environment variables for API keys and other settings. Use a `.env` file in the project root:

1. Copy `env.example` to `.env`:
   ```sh
   cp env.example .env
   ```
2. Edit `.env` and fill in the required values (e.g. `OPENROUTER_API_KEY`). See the comments in `env.example` for details.

# Stage 2 & beyond

Note: This section is for stages 2 and beyond.

1. Ensure you have `cargo (1.92)` installed locally.
2. Run `./your_program.sh` to run your program, which is implemented in
   `src/main.rs`. This command compiles your Rust project, so it might be slow
   the first time you run it. Subsequent runs will be fast.
3. Run `codecrafters submit` to submit your solution to CodeCrafters. Test
   output will be streamed to your terminal.
