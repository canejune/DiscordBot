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
                                 - `resume [session]`: Continue a specific session (e.g., `resume 20240101.md`).\n\
                                 - `summary [session]`: Get an AI summary of a specific session.\n\
                                 - `help`: Show this help message.\n\n\
                                 *Any other message will be treated as a chat input for the current session.*";
                let _ = msg.channel_id.say(&ctx.http, help_text).await;
                return;
            }
            "new" => {
                let mut sessions = self.active_sessions.lock().await;
                sessions.remove(&msg.channel_id);
                let _ = msg.channel_id.say(&ctx.http, "Started a new session! 🆕").await;
                log_to_file("STATUS", "User started a new session.").await;
                return;
            }
            "list" => {
                let mut response = String::from("**Session List:**\n");
                if let Ok(mut entries) = fs::read_dir("sessions").await {
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
                    let path = format!("sessions/{}", session_name);
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
                    let path = format!("sessions/{}", session_name);
                    if let Ok(history) = fs::read_to_string(&path).await {
                        let _ = msg.react(&ctx.http, '👀').await;
                        self.queue_size.fetch_add(1, Ordering::SeqCst);
                        let request = GeminiRequest {
                            ctx: ctx.clone(),
                            msg: msg.clone(),
                            session_path: path,
                            content: "Summarize the above conversation history in a concise way.".to_string(),
                            history,
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
                    if let Ok(mut entries) = fs::read_dir("sessions").await {
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
        
        let history = match fs::read_to_string(&session_path).await {
            Ok(h) => {
                log_to_file("INFO", &format!("Loaded context from {}: {} bytes", session_path, h.len())).await;
                h
            }
            Err(e) => {
                log_to_file("ERROR", &format!("Failed to read session file {}: {}", session_path, e)).await;
                String::new()
            }
        };

        let is_first_message = history.contains("# New Session") && !history.contains("User:");

        self.queue_size.fetch_add(1, Ordering::SeqCst);
        let request = GeminiRequest {
            ctx,
            msg,
            session_path,
            content: content_str,
            history,
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
