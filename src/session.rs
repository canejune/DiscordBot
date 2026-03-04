use tokio::fs;
use serenity::model::id::ChannelId;

pub async fn get_or_create_session(active_sessions: &tokio::sync::Mutex<std::collections::HashMap<ChannelId, String>>, channel_id: ChannelId) -> String {
    let mut sessions = active_sessions.lock().await;
    if let Some(path) = sessions.get(&channel_id) {
        if fs::metadata(path).await.is_ok() {
            return path.clone();
        }
    }

    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let channel_dir = format!("workspace/sessions/{}", channel_id);
    let _ = fs::create_dir_all(&channel_dir).await;
    let path = format!("{}/{}.md", channel_dir, timestamp);
    let _ = fs::write(&path, "# New Session\n\n").await;
    
    sessions.insert(channel_id, path.clone());
    path
}
