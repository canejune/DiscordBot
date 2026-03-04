---
name: github
description: Use this skill to interact with GitHub repositories, including pushing changes.
---

# GitHub Skill

This skill allows the agent to interact with GitHub repositories using stored credentials.

## 🛠 Usage

1.  **Identify Github ID:** Use `GITHUB_USERNAME` from the `.env` file.
2.  **Locate PAT:** The Personal Access Token is stored in the `.env` file as `GITHUB_PAT`.
3.  **Construct Push Command:** When pushing changes to GitHub, use the following format:
    ```bash
    git push https://<GITHUB_USERNAME>:<GITHUB_PAT>@github.com/<REPOSITORY_PATH>.git <BRANCH_NAME>
    ```

### Example
To push the `main` branch to the current repository:
```bash
# Load the credentials from .env
source .env
# Construct the URL with credentials
git push "https://$GITHUB_USERNAME:$GITHUB_PAT@github.com/canejune/DiscordBot.git" main
```

## 📋 Requirements

-   `.env` file must contain `GITHUB_PAT` and `GITHUB_USERNAME`.
-   Git must be installed and configured.
