use serenity::client::Context;
use serenity::model::channel::Message;
use serde::{Deserialize, Serialize};
use serenity::model::id::ChannelId;
use std::collections::HashMap;

pub struct GeminiRequest {
    pub ctx: Context,
    pub msg: Message,
    pub session_path: String,
    pub soul_path: Option<String>,
    pub workspace_path: Option<String>,
    pub content: String,
    pub is_first_message: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct BotState {
    pub active_sessions: HashMap<ChannelId, String>,
    pub workspace_folders: HashMap<ChannelId, String>,
}
