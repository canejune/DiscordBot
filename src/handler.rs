use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs;
use tokio::sync::{mpsc, Mutex};
use tokio::process::Command;
use crate::types::GeminiRequest;
use crate::utils::log_to_file;
use crate::session::get_or_create_session;
use serde_json;
use chrono::{DateTime, Utc};

pub struct Handler {
    pub active_sessions: Mutex<HashMap<ChannelId, String>>,
    pub workspace_folders: Mutex<HashMap<ChannelId, String>>,
    pub queue_tx: mpsc::Sender<GeminiRequest>,
    pub queue_size: Arc<AtomicUsize>,
    pub waiting_for_restart: Mutex<HashSet<ChannelId>>,
    pub start_time: DateTime<Utc>,
}

impl Handler {
    async fn save_state(&self) {
        let active_sessions = self.active_sessions.lock().await.clone();
        let workspace_folders = self.workspace_folders.lock().await.clone();
        let state = crate::types::BotState {
            active_sessions,
            workspace_folders,
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = fs::write("workspace/sessions/state.json", json).await;
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let content_str = msg.content.trim().to_string();
        let parts: Vec<&str> = content_str.split_whitespace().collect();
        let command = parts.get(0).map(|s| s.to_lowercase()).unwrap_or_default();

        // Check if waiting for restart confirmation
        {
            let mut waiting = self.waiting_for_restart.lock().await;
            if waiting.contains(&msg.channel_id) {
                if content_str.to_lowercase() == "yes" {
                    println!("Restart initiated by {} in channel {}", msg.author.name, msg.channel_id);
                    let _ = msg.channel_id.say(&ctx.http, "Restarting... 🔄").await;
                    
                    // Spawn a new instance of the current executable
                    match std::env::current_exe() {
                        Ok(exe) => {
                            let args: Vec<String> = std::env::args().skip(1).collect();
                            println!("Spawning new process: {:?} with args: {:?}", exe, args);
                            match std::process::Command::new(exe)
                                .args(args)
                                .spawn() 
                            {
                                Ok(_) => {
                                    println!("New process spawned successfully. Exiting current process...");
                                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                    std::process::exit(0);
                                },
                                Err(e) => {
                                    eprintln!("Failed to spawn new process: {}", e);
                                    let _ = msg.channel_id.say(&ctx.http, format!("Failed to restart: could not spawn new process ({}).", e)).await;
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to get current executable path: {}", e);
                            let _ = msg.channel_id.say(&ctx.http, format!("Failed to restart: could not find executable path ({}).", e)).await;
                        }
                    }
                    waiting.remove(&msg.channel_id);
                    return;
                } else {
                    waiting.remove(&msg.channel_id);
                    let _ = msg.channel_id.say(&ctx.http, "Restart cancelled.").await;
                    return; // Don't process this message further
                }
            }
        }

        match command.as_str() {
            "help" => {
                let help_text = "**Gemini Bot Commands:**\n\
                                 - `new`: Start a new conversation session.\n\
                                 - `list`: Show saved session files for this channel.\n\
                                 - `resume [session]`: Continue a specific session.\n\
                                 - `summary [session]`: Get an AI summary of a session.\n\
                                 - `workspace [path]`: Set a folder for AI context.\n\
                                 - `restart`: Restart the bot with confirmation.\n\
                                 - `info`: Show bot info, system, and network.\n\
                                 - `help`: Show this help message.\n\n\
                                 *Any other message will be treated as chat input.*";
                let _ = msg.channel_id.say(&ctx.http, help_text).await;
                return;
            }
            "info" => {
                let now = Utc::now();
                let uptime = now.signed_duration_since(self.start_time);
                let days = uptime.num_days();
                let hours = uptime.num_hours() % 24;
                let minutes = uptime.num_minutes() % 60;
                let seconds = uptime.num_seconds() % 60;
                let uptime_str = format!("{}d {}h {}m {}s", days, hours, minutes, seconds);
                let boot_time_str = self.start_time.format("%Y-%m-%d %H:%M:%S UTC").to_string();

                let sys_info = match Command::new("uname").arg("-a").output().await {
                    Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                    Err(_) => "Could not retrieve system info.".to_string(),
                };

                let mem_info = match Command::new("free").arg("-h").output().await {
                    Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                    Err(_) => "Could not retrieve memory info.".to_string(),
                };

                let net_info = match Command::new("ip").arg("addr").output().await {
                    Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                    Err(_) => "Could not retrieve network info.".to_string(),
                };

                let response = format!(
                    "**🤖 Bot Information**\n\
                    **Uptime:** {}\n\
                    **Boot Time:** `{}`\n\n\
                    **💻 System Information**\n\
                    ```\n\
                    OS: {}\n\n\
                    Memory:\n{}\n\
                    ```\n\n\
                    **🌐 Network Information**\n\
                    ```\n\
                    {}\n\
                    ```",
                    uptime_str, boot_time_str, sys_info.trim(), mem_info.trim(), net_info.trim()
                );

                if response.len() > 2000 {
                    let parts = vec![
                        format!("**🤖 Bot Information**\n**Uptime:** {}\n**Boot Time:** `{}`", uptime_str, boot_time_str),
                        format!("**💻 System Information**\n```\nOS: {}\n\nMemory:\n{}\n```", sys_info.trim(), mem_info.trim()),
                        format!("**🌐 Network Information**\n```\n{}\n```", net_info.trim()),
                    ];
                    for part in parts {
                        let mut p = part;
                        if p.len() > 2000 {
                            p.truncate(1997);
                            p.push_str("...");
                        }
                        let _ = msg.channel_id.say(&ctx.http, p).await;
                    }
                } else {
                    let _ = msg.channel_id.say(&ctx.http, response).await;
                }
                return;
            }
            "restart" => {
                {
                    let mut waiting = self.waiting_for_restart.lock().await;
                    waiting.insert(msg.channel_id);
                }
                let _ = msg.channel_id.say(&ctx.http, "Are you sure you want to restart the bot? Type `yes` to confirm.").await;
                return;
            }
            "new" => {
                {
                    let mut sessions = self.active_sessions.lock().await;
                    sessions.remove(&msg.channel_id);
                    let mut workspaces = self.workspace_folders.lock().await;
                    workspaces.remove(&msg.channel_id);
                }
                self.save_state().await;
                let _ = msg.channel_id.say(&ctx.http, "Started a new session! 🆕").await;
                log_to_file("STATUS", "User started a new session.").await;
                return;
            }
            "workspace" => {
                if let Some(path) = parts.get(1) {
                    {
                        let mut workspaces = self.workspace_folders.lock().await;
                        workspaces.insert(msg.channel_id, path.to_string());
                    }
                    self.save_state().await;
                    let _ = msg.channel_id.say(&ctx.http, format!("Workspace folder set to: `{}` 📂", path)).await;
                    log_to_file("STATUS", &format!("User set workspace: {}", path)).await;
                } else {
                    let _ = msg.channel_id.say(&ctx.http, "Usage: `workspace [path]`").await;
                }
                return;
            }
            "list" => {
                let mut response = String::from("**Session List for this channel:**\n");
                let channel_dir = format!("workspace/sessions/{}", msg.channel_id);
                if let Ok(mut entries) = fs::read_dir(&channel_dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let file_name = entry.file_name().to_string_lossy().into_owned();
                        if file_name.ends_with(".md") {
                            if let Ok(file_content) = fs::read_to_string(entry.path()).await {
                                let first_line = file_content.lines().next().unwrap_or("No Title").replace("# ", "");
                                response.push_str(&format!("- `{}`: {}\n", file_name, first_line));
                            }
                        }
                    }
                }
                if response == "**Session List for this channel:**\n" {
                    response.push_str("No sessions found for this channel.");
                }
                let _ = msg.channel_id.say(&ctx.http, response).await;
                return;
            }
            "resume" => {
                if let Some(session_name) = parts.get(1) {
                    let path = format!("workspace/sessions/{}/{}", msg.channel_id, session_name);
                    if fs::metadata(&path).await.is_ok() {
                        {
                            let mut sessions = self.active_sessions.lock().await;
                            sessions.insert(msg.channel_id, path.clone());
                        }
                        self.save_state().await;
                        let _ = msg.channel_id.say(&ctx.http, format!("Resumed session: `{}` 🔄", session_name)).await;
                        log_to_file("STATUS", &format!("User resumed session: {}", session_name)).await;
                    } else {
                        let _ = msg.channel_id.say(&ctx.http, format!("Error: Session file `{}` not found in this channel.", session_name)).await;
                    }
                } else {
                    let _ = msg.channel_id.say(&ctx.http, "Usage: `resume [session_filename]`").await;
                }
                return;
            }
            "summary" => {
                if let Some(session_name) = parts.get(1) {
                    let path = format!("workspace/sessions/{}/{}", msg.channel_id, session_name);
                    if fs::metadata(&path).await.is_ok() {
                        let _ = msg.react(&ctx.http, '👀').await;
                        self.queue_size.fetch_add(1, Ordering::SeqCst);
                        let workspaces = self.workspace_folders.lock().await;
                        let workspace_path = workspaces.get(&msg.channel_id).cloned();

                        let soul_path = if fs::metadata("workspace/SOUL.md").await.is_ok() {
                            Some("workspace/SOUL.md".to_string())
                        } else {
                            None
                        };

                        let request = GeminiRequest {
                            ctx: ctx.clone(),
                            msg: msg.clone(),
                            session_path: path,
                            soul_path,
                            workspace_path,
                            content: "Summarize the above conversation history in a concise way.".to_string(),
                            is_first_message: false,
                        };
                        if let Err(e) = self.queue_tx.send(request).await {
                            log_to_file("ERROR", &format!("Failed to send summary request to queue: {}", e)).await;
                            self.queue_size.fetch_sub(1, Ordering::SeqCst);
                        }
                    } else {
                        let _ = msg.channel_id.say(&ctx.http, format!("Error: Session file `{}` not found in this channel.", session_name)).await;
                    }
                } else {
                    let _ = msg.channel_id.say(&ctx.http, "Usage: `summary [session_filename]`").await;
                }
                return;
            }
            _ => {
                if content_str == "new session" {
                    {
                        let mut sessions = self.active_sessions.lock().await;
                        sessions.remove(&msg.channel_id);
                    }
                    self.save_state().await;
                    let _ = msg.channel_id.say(&ctx.http, "Started a new session! 🆕").await;
                    log_to_file("STATUS", "User started a new session.").await;
                    return;
                }
                if content_str == "session list" {
                    let mut response = String::from("**Session List for this channel:**\n");
                    let channel_dir = format!("workspace/sessions/{}", msg.channel_id);
                    if let Ok(mut entries) = fs::read_dir(&channel_dir).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            let file_name = entry.file_name().to_string_lossy().into_owned();
                            if file_name.ends_with(".md") {
                                if let Ok(file_content) = fs::read_to_string(entry.path()).await {
                                    let first_line = file_content.lines().next().unwrap_or("No Title").replace("# ", "");
                                    response.push_str(&format!("- `{}`: {}\n", file_name, first_line));
                                }
                            }
                        }
                    }
                    if response == "**Session List for this channel:**\n" {
                        response.push_str("No sessions found for this channel.");
                    }
                    let _ = msg.channel_id.say(&ctx.http, response).await;
                    return;
                }
            }
        }

        let current_size = self.queue_size.load(Ordering::SeqCst);
        if current_size >= 3 {
            let _ = msg.react(&ctx.http, '⏳').await;
            let _ = msg.channel_id.say(&ctx.http, "I'm currently busy, please wait a moment! (Queue is full: 3/3) ⏳").await;
            return;
        }

        let _ = msg.react(&ctx.http, '👀').await;
        log_to_file("INPUT", &format!("User: {}, Content: {}", msg.author.name, content_str)).await;

        let session_path = {
            let sessions = self.active_sessions.lock().await;
            if let Some(path) = sessions.get(&msg.channel_id) {
                if fs::metadata(path).await.is_ok() {
                    path.clone()
                } else {
                    drop(sessions);
                    let path = get_or_create_session(&self.active_sessions, msg.channel_id).await;
                    self.save_state().await;
                    path
                }
            } else {
                drop(sessions);
                let path = get_or_create_session(&self.active_sessions, msg.channel_id).await;
                self.save_state().await;
                path
            }
        };
        
        let is_first_message = if let Ok(metadata) = fs::metadata(&session_path).await {
            metadata.len() < 100 // Simple check: small files are likely new
        } else {
            true
        };

        let workspaces = self.workspace_folders.lock().await;
        let workspace_path = workspaces.get(&msg.channel_id).cloned();

        let soul_path = if fs::metadata("workspace/SOUL.md").await.is_ok() {
            Some("workspace/SOUL.md".to_string())
        } else {
            None
        };

        self.queue_size.fetch_add(1, Ordering::SeqCst);
        let request = GeminiRequest {
            ctx,
            msg,
            session_path,
            soul_path,
            workspace_path,
            content: content_str,
            is_first_message,
        };

        if let Err(e) = self.queue_tx.send(request).await {
            log_to_file("ERROR", &format!("Failed to send request to queue: {}", e)).await;
            self.queue_size.fetch_sub(1, Ordering::SeqCst);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}
