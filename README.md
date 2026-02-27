# Gemini Discord Bot (Rust)

A powerful, modular Discord bot written in Rust that integrates with the Gemini AI CLI to provide intelligent, session-aware conversations with real-time streaming.

## 🚀 Features

- **Gemini AI Integration**: Uses the `gemini` CLI for high-quality AI responses.
- **Real-time Streaming**: Watch the AI's response appear in Discord as it's being generated.
- **Session Management**: Rich commands to manage conversation history:
  - `help`: Show the command guide.
  - `new`: Reset the conversation and start fresh.
  - `list`: View all previously saved session files.
  - `resume [session]`: Continue a previous session from its file.
  - `summary [session]`: Get an AI-generated summary of a specific session.
- **Queue System**: Handles concurrent requests efficiently with a sequential processing queue (up to 3 pending).
- **Interactive Feedback**: Uses emoji reactions to show status:
  - 👀: Processing your request.
  - ⏳: Reached queue limit, please wait.
  - ✅: Successfully responded.
  - ❌: Encountered an error.
- **Robust Logging**: Detailed logging of inputs, outputs, and system status in `bot.log`.
  - Console: Clean, truncated output.
  - File: Full prompt and response data for debugging.

## 🛠 Architecture

The project is modularized into several components for better maintainability:

- `src/main.rs`: Entry point and background worker initialization.
- `src/handler.rs`: Discord event handling and command parsing.
- `src/gemini.rs`: AI processing and asynchronous output streaming.
- `src/session.rs`: Logic for creating and retrieving persistent session files.
- `src/types.rs`: Shared data structures.
- `src/utils.rs`: Shared utility functions (logging, message splitting).

## 📦 Setup & Running

1. **Prerequisites**:
   - Rust (latest stable)
   - [Gemini CLI](https://geminicli.com/) installed and configured.
   - A Discord Bot Token.

2. **Environment Variables**:
   Create a `.env` file in the root directory:
   ```env
   DISCORD_TOKEN=your_token_here
   ```

3. **Run the Bot**:
   ```bash
   cargo run
   ```

## 📝 Logging
The bot maintains a `bot.log` file. You can monitor it in real-time:
```bash
tail -f bot.log
```
