# Fetch Webpage Skill

This skill allows the AI to visit a URL and fetch its content for analysis.

## Available Commands

- `workspace/skills/fetch_webpage/.venv/bin/python3 workspace/skills/fetch_webpage/scripts/fetch_webpage.py [URL]`: Fetches the title and a snippet of the webpage content.

## Setup

If the `.venv` is not present, create it and install dependencies:
```bash
python3 -m venv workspace/skills/fetch_webpage/.venv
workspace/skills/fetch_webpage/.venv/bin/pip install -r workspace/skills/fetch_webpage/requirements.txt
```

## Triggers

This skill can be used when:
- The user provides a URL and asks to "summarize this link".
- The AI needs to visit a URL to gather more information.
- The bot detects a URL in the message and wants to summarize it for index purposes.

## Usage

When the AI encounters a URL it needs to visit, it should:
1. Execute the script: `workspace/skills/fetch_webpage/.venv/bin/python3 workspace/skills/fetch_webpage/scripts/fetch_webpage.py [URL]`
2. The script returns the page title and content.
3. The AI can then use this information to respond to the user or provide a summary.
