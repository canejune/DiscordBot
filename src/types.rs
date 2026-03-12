use serenity::http::Http;
use serenity::model::channel::Message;
use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct GeminiRequest {
    pub http: Arc<Http>,
    pub channel_id: ChannelId,
    pub user_name: String,
    pub msg: Option<Message>,
    pub session_path: String,
    pub soul_path: Option<String>,
    pub workspace_path: Option<String>,
    pub content: String,
    pub is_first_message: bool,
    pub attachment_paths: Vec<String>,
    pub is_indexing: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub prompt: String,
    pub interval: Option<u64>, // Interval in seconds
}

#[derive(Serialize, Deserialize)]
pub struct TaskList {
    pub tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScheduledTask {
    pub task_id: String,
    pub channel_id: ChannelId,
    pub session_path: String,
    pub workspace_path: Option<String>,
    pub last_run: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct BotState {
    pub active_sessions: HashMap<ChannelId, String>,
    pub workspace_folders: HashMap<ChannelId, String>,
    pub scheduled_tasks: Vec<ScheduledTask>,
}
