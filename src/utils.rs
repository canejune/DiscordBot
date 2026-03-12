use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

pub async fn log_to_file(level: &str, content: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_entry = format!("[{}] [{}] {}\n", timestamp, level, content);
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("bot.log")
        .await
    {
        let _ = file.write_all(log_entry.as_bytes()).await;
    }
    
    println!("{}", log_entry.trim());
}

pub fn split_message(text: &str, limit: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for line in text.lines() {
        if current_chunk.len() + line.len() + 1 > limit {
            if !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = String::new();
            }
            
            if line.len() > limit {
                let mut line_str = line.to_string();
                while line_str.len() > limit {
                    chunks.push(line_str.drain(..limit).collect());
                }
                current_chunk = line_str;
            } else {
                current_chunk.push_str(line);
                current_chunk.push('\n');
            }
        } else {
            current_chunk.push_str(line);
            current_chunk.push('\n');
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}
