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
use crate::types::GeminiRequest;
use crate::utils::log_to_file;
use crate::session::get_or_create_session;

pub struct Handler {
    pub active_sessions: Mutex<HashMap<ChannelId, String>>,
    pub workspace_folders: Mutex<HashMap<ChannelId, String>>,
    pub queue_tx: mpsc::Sender<GeminiRequest>,
    pub queue_size: Arc<AtomicUsize>,
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
                                 - `list`: Show all saved session files.\n\
                                 - `resume [session]`: Continue a specific session.\n\
                                 - `summary [session]`: Get an AI summary of a session.\n\
                                 - `workspace [path]`: Set a folder for AI context.\n\
                                 - `help`: Show this help message.\n\n\
                                 *Any other message will be treated as chat input.*";
                let _ = msg.channel_id.say(&ctx.http, help_text).await;
                return;
            }
            "new" => {
                let mut sessions = self.active_sessions.lock().await;
                sessions.remove(&msg.channel_id);
                let mut workspaces = self.workspace_folders.lock().await;
                workspaces.remove(&msg.channel_id);
                let _ = msg.channel_id.say(&ctx.http, "Started a new session! 🆕").await;
                log_to_file("STATUS", "User started a new session.").await;
                return;
            }
            "workspace" => {
                if let Some(path) = parts.get(1) {
                    let mut workspaces = self.workspace_folders.lock().await;
                    workspaces.insert(msg.channel_id, path.to_string());
                    let _ = msg.channel_id.say(&ctx.http, format!("Workspace folder set to: `{}` 📂", path)).await;
                    log_to_file("STATUS", &format!("User set workspace: {}", path)).await;
                } else {
                    let _ = msg.channel_id.say(&ctx.http, "Usage: `workspace [path]`").await;
                }
                return;
            }
            "list" => {
                let mut response = String::from("**Session List:**\n");
                if let Ok(mut entries) = fs::read_dir("workspace/sessions").await {
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
                if response == "**Session List:**\n" {
                    response.push_str("No sessions found.");
                }
                let _ = msg.channel_id.say(&ctx.http, response).await;
                return;
            }
            "resume" => {
                if let Some(session_name) = parts.get(1) {
                    let path = format!("workspace/sessions/{}", session_name);
                    if fs::metadata(&path).await.is_ok() {
                        let mut sessions = self.active_sessions.lock().await;
                        sessions.insert(msg.channel_id, path.clone());
                        let _ = msg.channel_id.say(&ctx.http, format!("Resumed session: `{}` 🔄", session_name)).await;
                        log_to_file("STATUS", &format!("User resumed session: {}", session_name)).await;
                    } else {
                        let _ = msg.channel_id.say(&ctx.http, format!("Error: Session file `{}` not found.", session_name)).await;
                    }
                } else {
                    let _ = msg.channel_id.say(&ctx.http, "Usage: `resume [session_filename]`").await;
                }
                return;
            }
            "summary" => {
                if let Some(session_name) = parts.get(1) {
                    let path = format!("workspace/sessions/{}", session_name);
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
                        let _ = msg.channel_id.say(&ctx.http, format!("Error: Session file `{}` not found.", session_name)).await;
                    }
                } else {
                    let _ = msg.channel_id.say(&ctx.http, "Usage: `summary [session_filename]`").await;
                }
                return;
            }
            _ => {
                if content_str == "new session" {
                    let mut sessions = self.active_sessions.lock().await;
                    sessions.remove(&msg.channel_id);
                    let _ = msg.channel_id.say(&ctx.http, "Started a new session! 🆕").await;
                    log_to_file("STATUS", "User started a new session.").await;
                    return;
                }
                if content_str == "session list" {
                    let mut response = String::from("**Session List:**\n");
                    if let Ok(mut entries) = fs::read_dir("workspace/sessions").await {
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
                    if response == "**Session List:**\n" {
                        response.push_str("No sessions found.");
                    }
                    let _ = msg.channel_id.say(&ctx.http, response).await;
                    return;
                }
            }
        }

        let current_size = self.queue_size.load(Ordering::SeqCst);
        if current_size >= 3 {
            let _ = msg.react(&ctx.http, '⏳').await;
            let _ = msg.channel_id.say(&ctx.http, "지금 열심히 일하고 있으니 잠시만 기다려주세요! (대기열이 가득 찼습니다. 3/3) ⏳").await;
            return;
        }

        let _ = msg.react(&ctx.http, '👀').await;
        log_to_file("INPUT", &format!("User: {}, Content: {}", msg.author.name, content_str)).await;

        let session_path = get_or_create_session(&self.active_sessions, msg.channel_id).await;
        
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
