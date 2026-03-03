use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{interval, Duration, Instant};
use crate::types::GeminiRequest;
use crate::utils::split_message;

pub async fn process_gemini_request(req: GeminiRequest, queue_size: Arc<AtomicUsize>) {
    let ctx = req.ctx;
    let msg = req.msg;
    let session_path = req.session_path;
    let content = req.content;
    let is_first_message = req.is_first_message;

    let system_instruction = "You are a helpful Discord bot. Above is the conversation history for context. \
                             Do NOT repeat previous answers or the 'Gemini:' prefix in your response. \
                             Your task is to respond specifically to the message below using the history (provided via stdin) for context.";
    
    let full_prompt = format!(
        "{}\n\n[Latest Message]\nUser: {}\nGemini: ", 
        system_instruction,
        content
    );

    let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

    let mut command = Command::new("gemini");
    command.arg("-y");
    
    if let Some(ref path) = req.workspace_path {
        command.arg("--include-directories").arg(path);
    }
    
    command.arg("-p").arg(&full_prompt);
    
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let console_log = format!("[{}] [EXEC] gemini -y -p \"[PROMPT_CONTENT]\" (Prompt length: {} bytes)", timestamp, full_prompt.len());
    let file_log = format!("[{}] [EXEC] gemini -y -p \"{}\"\n", timestamp, full_prompt);
    
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("bot.log") {
        use std::io::Write;
        let _ = file.write_all(file_log.as_bytes());
    }
    println!("{}", console_log);

    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    command.stdin(std::process::Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            let err_msg = format!("Failed to spawn Gemini CLI: {}", e);
            eprintln!("{}", err_msg);
            let _ = msg.delete_reaction_emoji(&ctx.http, '👀').await;
            let _ = msg.react(&ctx.http, '❌').await;
            let _ = msg.channel_id.say(&ctx.http, &err_msg).await;
            queue_size.fetch_sub(1, Ordering::SeqCst);
            return;
        }
    };

    // Pipe SOUL.md and session file to stdin
    if let Some(mut stdin) = child.stdin.take() {
        if let Some(ref s_path) = req.soul_path {
            if let Ok(soul_content) = fs::read_to_string(s_path).await {
                let _ = stdin.write_all(format!("{}\n\n", soul_content).as_bytes()).await;
            }
        }
        if let Ok(history) = fs::read_to_string(&session_path).await {
            let _ = stdin.write_all(history.as_bytes()).await;
        }
    }

    let mut stdout_reader = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut stderr_reader = BufReader::new(child.stderr.take().unwrap()).lines();

    let mut final_stdout = String::new();
    let mut final_stderr = String::new();
    let mut buffer = String::new();
    let mut last_send = Instant::now();
    let mut heartbeat_interval = interval(Duration::from_secs(10));

    let mut stdout_done = false;
    let mut stderr_done = false;

    while !stdout_done || !stderr_done {
        tokio::select! {
            _ = heartbeat_interval.tick() => {
                let _ = msg.channel_id.broadcast_typing(&ctx.http).await;
            }
            line = stdout_reader.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(l)) => {
                        final_stdout.push_str(&l);
                        final_stdout.push('\n');
                        buffer.push_str(&l);
                        buffer.push('\n');
                        
                        if buffer.len() > 1000 || last_send.elapsed().as_secs() > 3 {
                            if !buffer.trim().is_empty() {
                                for chunk in split_message(&buffer, 1900) {
                                    let _ = msg.channel_id.say(&ctx.http, chunk).await;
                                }
                                buffer.clear();
                                last_send = Instant::now();
                            }
                        }
                    }
                    _ => stdout_done = true,
                }
            }
            line = stderr_reader.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(l)) => {
                        final_stderr.push_str(&l);
                        final_stderr.push('\n');
                    }
                    _ => stderr_done = true,
                }
            }
        }
    }

    if !buffer.trim().is_empty() {
        for chunk in split_message(&buffer, 1900) {
            let _ = msg.channel_id.say(&ctx.http, chunk).await;
        }
    }

    let status = child.wait().await;

    match status {
        Ok(s) if s.success() => {
            let mut final_response = final_stdout.clone();

            if final_response.trim().is_empty() && !final_stderr.trim().is_empty() {
                final_response = format!("CLI Output (from stderr): {}", final_stderr);
            }

            let final_response_trimmed = final_response.trim();

            if !final_response_trimmed.is_empty() {
                match OpenOptions::new().append(true).open(&session_path).await {
                    Ok(mut file) => {
                        let _ = file.write_all(format!("\nUser: {}\nGemini: {}\n", content, final_response_trimmed).as_bytes()).await;
                    }
                    Err(e) => {
                        eprintln!("Failed to open session file: {}", e);
                    }
                }
            } else {
                let _ = msg.channel_id.say(&ctx.http, "Gemini finished its task, but no response was generated. 😶").await;
            }

            if is_first_message {
                let summary = content.chars().take(30).collect::<String>().trim().to_string();
                if let Ok(file_content) = fs::read_to_string(&session_path).await {
                    let updated_content = file_content.replace("# New Session", &format!("# {}", summary));
                    let _ = fs::write(&session_path, updated_content).await;
                }
            }
            let _ = msg.delete_reaction_emoji(&ctx.http, '👀').await;
            let _ = msg.react(&ctx.http, '✅').await;
        }
        Ok(s) => {
            let _ = msg.delete_reaction_emoji(&ctx.http, '👀').await;
            let _ = msg.react(&ctx.http, '❌').await;
            let _ = msg.channel_id.say(&ctx.http, format!("Gemini CLI exited with failure: {}", s)).await;
            if !final_stderr.is_empty() {
                let _ = msg.channel_id.say(&ctx.http, format!("Stderr: ```\n{}\n```", final_stderr)).await;
            }
        }
        Err(e) => {
            let _ = msg.delete_reaction_emoji(&ctx.http, '👀').await;
            let _ = msg.react(&ctx.http, '❌').await;
            let _ = msg.channel_id.say(&ctx.http, format!("Process error: {}", e)).await;
        }
    }

    queue_size.fetch_sub(1, Ordering::SeqCst);
}
