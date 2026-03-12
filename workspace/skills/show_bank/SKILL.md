# Show Bank Skill

This skill allows you to display a summary of all files stored in the bank of the current channel by reading the `index.md` file.

## Available Commands

- `python3 workspace/skills/show_bank/scripts/show_bank.py`: Displays the file summaries from the current channel's `index.md`.

## Triggers

This skill can be used when the user asks for:
- "show files in the bank"
- "show stored files"
- "show uploaded files"
- "list files"
- "bank summary"

## Usage

When the user requests to see the bank files, execute the script:
`python3 workspace/skills/show_bank/scripts/show_bank.py`

The output will contain the content of the `index.md` file for the current channel.
