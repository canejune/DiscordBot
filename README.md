# Gemini Discord Bot (Rust)

A powerful, modular Discord bot written in Rust that integrates with the Gemini AI CLI to provide intelligent, session-aware conversations with real-time streaming and extensible skills.

## 🚀 Features

- **Gemini AI Integration**: Uses the `gemini` CLI for high-quality AI responses.
- **Workspace Support**: Advanced context handling using a dedicated `workspace` directory.
- **AI Soul (`SOUL.md`)**: Configurable personality and behavioral instructions for the AI.
- **Extensible Skills**: Modular skill system (e.g., `get_stock_price`) that allows the AI to perform specialized tasks using external scripts.
- **Real-time Streaming**: Watch the AI's response appear in Discord as it's being generated.
- **Session Management**: Rich commands to manage conversation history with per-channel isolation:
  - `help`: Show the command guide.
  - `new`: Reset the conversation and start fresh.
  - `list`: View session files for the current channel.
  - `resume [session]`: Continue a previous session from its file.
  - `summary [session]`: Get an AI-generated summary of a specific session.
  - `workspace [path]`: Set a specific folder for AI context (channel-specific).
  - `restart`: Restart the bot with confirmation.
  - `info`: Show detailed bot information, system status, and network info.
- **State Persistence**: Automatically saves and restores active sessions and workspace settings across bot restarts using `state.json`.
- **Queue System**: Handles concurrent requests efficiently with a sequential processing queue (up to 3 pending).
- **Interactive Feedback**: Uses emoji reactions to show status:
  - 👀: Processing your request.
  - ⏳: Reached queue limit, please wait.
  - ✅: Successfully responded.
  - ❌: Encountered an error.
- **Robust Logging**: Detailed logging of inputs, outputs, and system status in `bot.log`.

## 📂 Project Structure

```text
/
├── src/                # Rust source code
├── workspace/          # AI Workspace
│   ├── SOUL.md         # AI instructions and personality
│   ├── sessions/       # Persistent session files
│   │   ├── state.json  # Saved bot state (sessions/workspaces)
│   │   └── {channel_id}/ # Per-channel session history
│   └── skills/         # Modular skill scripts (Python, etc.)
├── Cargo.toml          # Rust dependencies
└── README.md           # Documentation
```

## 🛠 Architecture

The project is modularized into several components for better maintainability:

- `src/main.rs`: Entry point and background worker initialization.
- `src/handler.rs`: Discord event handling and command parsing.
- `src/gemini.rs`: AI processing and asynchronous output streaming.
- `src/session.rs`: Logic for creating and retrieving persistent session files within the workspace.
- `src/types.rs`: Shared data structures.
- `src/utils.rs`: Shared utility functions (logging, message splitting).

## 📦 Setup & Running

1. **Prerequisites**:
   - Rust (latest stable)
   - [Gemini CLI](https://geminicli.com/) installed and configured.
   - Python 3 (for certain skills)
   - A Discord Bot Token.

2. **Environment Variables**:
   Create a `.env` file in the root directory:
   ```env
   DISCORD_TOKEN=your_token_here
   ```

3. **Skill Setup** (Optional):
   For skills like `get_stock_price`, install their specific dependencies:
   ```bash
   pip install -r workspace/skills/get_stock_price/requirements.txt
   ```

4. **Run the Bot**:
   ```bash
   cargo run
   ```

## 📝 Logging
The bot maintains a `bot.log` file. You can monitor it in real-time:
```bash
tail -f bot.log
```
