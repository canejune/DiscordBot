use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs;
use tokio::sync::{mpsc, Mutex};
use tokio::process::Command;
use crate::types::{GeminiRequest, ScheduledTask};
use crate::utils::log_to_file;
use crate::session::get_or_create_session;
use serde_json;
use chrono::{DateTime, Utc};

pub struct Handler {
    pub active_sessions: Mutex<HashMap<ChannelId, String>>,
    pub workspace_folders: Mutex<HashMap<ChannelId, String>>,
    pub scheduled_tasks: Arc<Mutex<Vec<ScheduledTask>>>,
    pub queue_tx: mpsc::Sender<GeminiRequest>,
    pub queue_size: Arc<AtomicUsize>,
    pub start_time: DateTime<Utc>,
}

impl Handler {
    async fn save_state(&self) {
        let active_sessions = self.active_sessions.lock().await.clone();
        let workspace_folders = self.workspace_folders.lock().await.clone();
        let scheduled_tasks = self.scheduled_tasks.lock().await.clone();
        let state = crate::types::BotState {
            active_sessions,
            workspace_folders,
            scheduled_tasks,
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

        match command.as_str() {
            "help" => {
                let help_text = "**Gemini Bot Commands:**\n\
                                 - `new`: Start a new conversation session.\n\
                                 - `list`: Show saved session files for this channel.\n\
                                 - `resume [session]`: Continue a specific session.\n\
                                 - `summary [session]`: Get an AI summary of a session.\n\
                                 - `trigger [id]`: Execute a predefined task. If it has an interval, it schedules it.\n\
                                 - `untrigger [id]`: Stop a scheduled task for this channel.\n\
                                 - `triggers` | `trigger-list`: List all available and active triggers.\n\
                                 - `workspace [path]`: Set a folder for AI context.\n\
                                 - `terminate`: Terminate the bot process.\n\
                                 - `info`: Show bot info, system, and network.\n\
                                 - `help`: Show this help message.\n\n\
                                 *Any other message will be treated as chat input.*";
                let _ = msg.channel_id.say(&ctx.http, help_text).await;
                return;
            }
            "triggers" | "trigger-list" => {
                let tasks_json = fs::read_to_string("workspace/tasks.json").await.unwrap_or_else(|_| "{\"tasks\": []}".to_string());
                let task_list: crate::types::TaskList = serde_json::from_str(&tasks_json).unwrap_or(crate::types::TaskList { tasks: vec![] });
                
                let mut response = String::from("**Available Trigger Tasks (from tasks.json):**\n");
                if task_list.tasks.is_empty() {
                    response.push_str("No tasks available.\n");
                } else {
                    for t in &task_list.tasks {
                        let interval_str = t.interval.map(|i| format!(" ({}s interval)", i)).unwrap_or_default();
                        response.push_str(&format!("- `{}`: {}{}\n", t.id, t.prompt, interval_str));
                    }
                }

                let scheduled = self.scheduled_tasks.lock().await;
                response.push_str("\n**Active Schedules for this channel:**\n");
                let mut count = 0;
                for s in scheduled.iter() {
                    if s.channel_id == msg.channel_id {
                        response.push_str(&format!("- `{}` (Last run: {})\n", s.task_id, s.last_run.format("%Y-%m-%d %H:%M:%S UTC")));
                        count += 1;
                    }
                }
                if count == 0 {
                    response.push_str("No active triggers scheduled.");
                }
                let _ = msg.channel_id.say(&ctx.http, response).await;
                return;
            }
            "untrigger" => {
                if let Some(task_id) = parts.get(1) {
                    let mut scheduled = self.scheduled_tasks.lock().await;
                    let initial_len = scheduled.len();
                    scheduled.retain(|s| !(s.task_id == *task_id && s.channel_id == msg.channel_id));
                    if scheduled.len() < initial_len {
                        drop(scheduled);
                        self.save_state().await;
                        let _ = msg.channel_id.say(&ctx.http, format!("Removed trigger `{}` from schedule. 🛑", task_id)).await;
                    } else {
                        let _ = msg.channel_id.say(&ctx.http, format!("Trigger `{}` was not scheduled for this channel.", task_id)).await;
                    }
                } else {
                    let _ = msg.channel_id.say(&ctx.http, "Usage: `untrigger [task_id]`").await;
                }
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
            "terminate" => {
                let _ = msg.channel_id.say(&ctx.http, "Terminating bot... 👋").await;
                println!("Termination initiated by {} in channel {}", msg.author.name, msg.channel_id);
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                std::process::exit(0);
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
                        let current_size = self.queue_size.load(Ordering::SeqCst);
                        if current_size >= 3 {
                            let _ = msg.react(&ctx.http, '⏳').await;
                            let _ = msg.channel_id.say(&ctx.http, "I'm currently busy, please wait a moment! (Queue is full: 3/3) ⏳").await;
                            return;
                        }

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
                            http: ctx.http.clone(),
                            channel_id: msg.channel_id,
                            user_name: msg.author.name.clone(),
                            msg: Some(msg.clone()),
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
            "trigger" => {
                println!("[DEBUG] Trigger command received: {:?}", parts);
                if let Some(task_id) = parts.get(1) {
                    let tasks_json = fs::read_to_string("workspace/tasks.json").await.unwrap_or_else(|_| "{\"tasks\": []}".to_string());
                    let task_list: crate::types::TaskList = serde_json::from_str(&tasks_json).unwrap_or(crate::types::TaskList { tasks: vec![] });
                    
                    let found_task = task_list.tasks.iter().find(|t| t.id == *task_id).cloned();

                    if let Some(task) = found_task {
                        println!("[DEBUG] Found task: {:?}", task.id);
                        let current_size = self.queue_size.load(Ordering::SeqCst);
                        if current_size >= 3 {
                            println!("[DEBUG] Queue full (3/3)");
                            let _ = msg.react(&ctx.http, '⏳').await;
                            let _ = msg.channel_id.say(&ctx.http, "I'm currently busy, please wait a moment! (Queue is full: 3/3) ⏳").await;
                            return;
                        }

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
                            metadata.len() < 100
                        } else {
                            true
                        };

                        let workspace_path = {
                            let workspaces = self.workspace_folders.lock().await;
                            workspaces.get(&msg.channel_id).cloned()
                        };

                        let soul_path = if fs::metadata("workspace/SOUL.md").await.is_ok() {
                            Some("workspace/SOUL.md".to_string())
                        } else {
                            None
                        };

                        // If it has an interval, schedule it
                        if let Some(interval) = task.interval {
                            println!("[DEBUG] Task has interval: {}s. Checking if already scheduled.", interval);
                            let is_already_scheduled = {
                                let scheduled = self.scheduled_tasks.lock().await;
                                scheduled.iter().any(|s| s.task_id == *task_id && s.channel_id == msg.channel_id)
                            };

                            if !is_already_scheduled {
                                println!("[DEBUG] Scheduling task for first time.");
                                {
                                    let mut scheduled = self.scheduled_tasks.lock().await;
                                    scheduled.push(ScheduledTask {
                                        task_id: task.id.clone(),
                                        channel_id: msg.channel_id,
                                        session_path: session_path.clone(),
                                        workspace_path: workspace_path.clone(),
                                        last_run: Utc::now(),
                                    });
                                }
                                println!("[DEBUG] Saving state after scheduling.");
                                self.save_state().await;
                                let _ = msg.channel_id.say(&ctx.http, format!("Task `{}` scheduled every {} seconds! 🕒", task_id, interval)).await;
                            } else {
                                println!("[DEBUG] Task already scheduled for this channel.");
                                let _ = msg.channel_id.say(&ctx.http, format!("Task `{}` is already scheduled for this channel.", task_id)).await;
                            }
                            // Don't return here anymore, let it run once immediately
                            println!("[DEBUG] Proceeding to immediate execution for triggered task.");
                        }

                        let _ = msg.react(&ctx.http, '👀').await;
                        self.queue_size.fetch_add(1, Ordering::SeqCst);
                        println!("[DEBUG] Preparing GeminiRequest for trigger: {}", task.id);
                        let request = GeminiRequest {
                            http: ctx.http.clone(),
                            channel_id: msg.channel_id,
                            user_name: msg.author.name.clone(),
                            msg: Some(msg.clone()),
                            session_path,
                            soul_path,
                            workspace_path,
                            content: task.prompt,
                            is_first_message,
                        };

                        if let Err(e) = self.queue_tx.send(request).await {
                            println!("[DEBUG] Failed to send to queue_tx: {}", e);
                            log_to_file("ERROR", &format!("Failed to send triggered request to queue: {}", e)).await;
                            self.queue_size.fetch_sub(1, Ordering::SeqCst);
                        } else {
                            println!("[DEBUG] Successfully sent to queue_tx");
                        }
                    } else {
                        println!("[DEBUG] Task ID not found: {}", task_id);
                        let _ = msg.channel_id.say(&ctx.http, format!("Error: Trigger ID `{}` not found.", task_id)).await;
                    }
                } else {
                    println!("[DEBUG] Trigger command missing task ID");
                    let _ = msg.channel_id.say(&ctx.http, "Usage: `trigger [task_id]`").await;
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
            http: ctx.http.clone(),
            channel_id: msg.channel_id,
            user_name: msg.author.name.clone(),
            msg: Some(msg),
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
