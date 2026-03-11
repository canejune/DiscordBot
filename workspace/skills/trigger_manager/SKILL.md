# Trigger Manager Skill

This skill allows you to manage predefined tasks (triggers) that can be executed by you or the user.

## Available Commands

- `python3 workspace/skills/trigger_manager/scripts/trigger_manager.py list`: List all available triggers.
- `python3 workspace/skills/trigger_manager/scripts/trigger_manager.py add <id> <prompt>`: Add a new trigger with the specified ID and prompt.
- `python3 workspace/skills/trigger_manager/scripts/trigger_manager.py remove <id>`: Remove the trigger with the specified ID.

## Usage

- To list triggers: `python3 workspace/skills/trigger_manager/scripts/trigger_manager.py list`
- To add a trigger: `python3 workspace/skills/trigger_manager/scripts/trigger_manager.py add daily_summary "Summarize all workspace activities today."`
- To remove a trigger: `python3 workspace/skills/trigger_manager/scripts/trigger_manager.py remove daily_summary`

## Triggering a Task

You can trigger a task autonomously by including `[[trigger:task_id]]` in your response. This will cause the bot to automatically execute the predefined prompt associated with `task_id`.
