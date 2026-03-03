use serenity::client::Context;
use serenity::model::channel::Message;

pub struct GeminiRequest {
    pub ctx: Context,
    pub msg: Message,
    pub session_path: String,
    pub soul_path: Option<String>,
    pub workspace_path: Option<String>,
    pub content: String,
    pub is_first_message: bool,
}
