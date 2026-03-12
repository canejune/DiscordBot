# Workspace Skills Summary

This file summarizes the available skills in the `workspace/skills` directory.

## Available Skills

| Skill Name | Description | Location |
| :--- | :--- | :--- |
| **get_stock_price** | Fetches real-time or historical stock price data for a given ticker symbol using the `yfinance` library. | `workspace/skills/get_stock_price/` |
| **github** | Handles authenticated Git operations (push, etc.) using GITHUB_USERNAME and GITHUB_PAT from the `.env` file. **Use this instead of standard git push.** | `workspace/skills/github/` |
| **show_bank** | Displays a summary of all files stored in the bank of the current channel by reading the `index.md` file. | `workspace/skills/show_bank/` |
| **trigger_manager** | Manages predefined tasks (triggers) that can be executed by you or the user. | `workspace/skills/trigger_manager/` |

---
Refer to the individual `SKILL.md` files in each skill directory for detailed usage instructions.
