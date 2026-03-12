# Download File Skill

This skill allows you to "push" or "send" files from the current channel's bank folder to the user over the chat.

## Available Commands

- `python3 workspace/skills/download_file/scripts/download_file.py [filename]`: This command tells the bot to upload a specific file from the bank to the Discord channel.

## Triggers

This skill can be used when the user asks for:
- "download [filename]"
- "send me [filename] from the bank"
- "give me the file [filename]"
- "post the file [filename]"

## Usage

When the user requests to download a file from the bank, you MUST:
1. Execute the script: `python3 workspace/skills/download_file/scripts/download_file.py [filename]`
2. The script returns the required tag: `[[download:filename]]`.
3. You **MUST** include this exact tag in your final response to the user so the bot can process the download and upload the file to Discord.
