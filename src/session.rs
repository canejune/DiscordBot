use tokio::fs;
use serenity::model::id::ChannelId;

pub async fn get_or_create_session(active_sessions: &tokio::sync::Mutex<std::collections::HashMap<ChannelId, String>>, channel_id: ChannelId, channel_dir: &str) -> String {
    let mut sessions = active_sessions.lock().await;
    if let Some(path) = sessions.get(&channel_id) {
        if fs::metadata(path).await.is_ok() {
            return path.clone();
        }
    }

    let sessions_dir = format!("{}/sessions", channel_dir);
    let _ = fs::create_dir_all(&sessions_dir).await;
    
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let path = format!("{}/{}.md", sessions_dir, timestamp);
    let _ = fs::write(&path, "# New Session\n\n").await;
    
    sessions.insert(channel_id, path.clone());
    path
}
