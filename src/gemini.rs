use serenity::all::{CreateAttachment, CreateMessage};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{interval, Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use crate::types::GeminiRequest;
use crate::utils::split_message;
use regex::Regex;

pub async fn process_gemini_request(
    req: GeminiRequest, 
    queue_size: Arc<AtomicUsize>,
    queue_tx: mpsc::Sender<GeminiRequest>,
    scheduled_tasks: Arc<Mutex<Vec<crate::types::ScheduledTask>>>
) {
    println!("[DEBUG] Processing GeminiRequest for channel: {}", req.channel_id);
    let http = req.http;
    let channel_id = req.channel_id;
    let user_name = req.user_name;
    let msg = req.msg;
    let session_path = req.session_path;
    let content = req.content;
    let is_first_message = req.is_first_message;

    println!("[DEBUG] Content: {}", content);

    let mut final_content = content.clone();
    if !req.attachment_paths.is_empty() {
        let mut file_contents = String::new();
        for file_path in &req.attachment_paths {
            if let Ok(content) = fs::read_to_string(file_path).await {
                file_contents.push_str(&format!("File Content of `{}`:\n```\n{}\n```\n\n", file_path, content));
            }
        }
        if !file_contents.is_empty() {
            final_content = format!("{}\nInstruction: {}", file_contents, content);
        }
    }

    let system_instruction = "You are a helpful Discord bot with access to various skills in your workspace. \
                             Above is the conversation history for context. \
                             Do NOT repeat previous answers or the 'Gemini:' prefix in your response. \
                             Your task is to respond specifically to the message below using the history (provided via stdin) for context. \
                             If you need to perform an action (like sending a file or triggering a task), you MUST include the appropriate tag in your final response: \
                             - To send a file from the bank: `[[download:filename]]` \
                             - To trigger a task: `[[trigger:task_id]]` \
                             You can use the provided skills to find filenames or task IDs.";
    
    let full_prompt = format!(
        "{}\n\n[Latest Message]\n{}: {}\nGemini: ", 
        system_instruction,
        user_name,
        final_content
    );

    let _ = channel_id.broadcast_typing(&http).await;

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
        Ok(child) => {
            println!("[DEBUG] Gemini CLI spawned successfully, PID: {:?}", child.id());
            child
        },
        Err(e) => {
            let err_msg = format!("Failed to spawn Gemini CLI: {}", e);
            eprintln!("{}", err_msg);
            if let Some(m) = msg {
                let _ = m.delete_reaction_emoji(&http, '👀').await;
                let _ = m.react(&http, '❌').await;
            }
            let _ = channel_id.say(&http, &err_msg).await;
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
                let _ = channel_id.broadcast_typing(&http).await;
            }
            line = stdout_reader.next_line(), if !stdout_done => {
                match line {
                    Ok(Some(l)) => {
                        println!("[DEBUG] CLI stdout: {}", l);
                        final_stdout.push_str(&l);
                        final_stdout.push('\n');
                        buffer.push_str(&l);
                        buffer.push('\n');
                        
                        if buffer.len() > 1000 || last_send.elapsed().as_secs() > 3 {
                            if !buffer.trim().is_empty() {
                                println!("[DEBUG] Sending chunk to Discord ({} bytes)", buffer.len());
                                for chunk in split_message(&buffer, 1900) {
                                    let _ = channel_id.say(&http, chunk).await;
                                }
                                buffer.clear();
                                last_send = Instant::now();
                            }
                        }
                    }
                    _ => {
                        println!("[DEBUG] CLI stdout reached EOF");
                        stdout_done = true;
                    },
                }
            }
            line = stderr_reader.next_line(), if !stderr_done => {
                match line {
                    Ok(Some(l)) => {
                        println!("[DEBUG] CLI stderr: {}", l);
                        final_stderr.push_str(&l);
                        final_stderr.push('\n');
                    }
                    _ => {
                        println!("[DEBUG] CLI stderr reached EOF");
                        stderr_done = true;
                    },
                }
            }
        }
    }

    if !buffer.trim().is_empty() {
        println!("[DEBUG] Sending final chunk to Discord ({} bytes)", buffer.len());
        for chunk in split_message(&buffer, 1900) {
            let _ = channel_id.say(&http, chunk).await;
        }
    }

    let status = child.wait().await;
    println!("[DEBUG] Gemini CLI process exited with status: {:?}", status);

    match status {
        Ok(s) if s.success() => {
            let mut final_response = final_stdout.clone();

            if final_response.trim().is_empty() && !final_stderr.trim().is_empty() {
                final_response = format!("CLI Output (from stderr): {}", final_stderr);
            }

            let final_response_trimmed = final_response.trim();

            if !final_response_trimmed.is_empty() {
                if req.is_indexing {
                    // Update index.md
                    if let Ok(channel_name) = channel_id.name(&http).await {
                        let sanitized = crate::utils::sanitize_filename(&channel_name);
                        let index_path = format!("workspace/channels/{}/index.md", sanitized);
                        
                        let mut index_file = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&index_path)
                            .await
                            .unwrap();
                        
                        if let Ok(metadata) = fs::metadata(&index_path).await {
                            if metadata.len() == 0 {
                                let _ = index_file.write_all(b"# Channel File Index\n\n").await;
                            }
                        }
                        
                        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                        let index_entry_content = if let Some(pos) = final_response_trimmed.find('[') {
                            &final_response_trimmed[pos..]
                        } else {
                            final_response_trimmed
                        };
                        let index_entry = format!("- [{}] {}\n", timestamp, index_entry_content);
                        let _ = index_file.write_all(index_entry.as_bytes()).await;
                    }
                }

                match OpenOptions::new().append(true).open(&session_path).await {
                    Ok(mut file) => {
                        let _ = file.write_all(format!("\nUser: {}\nGemini: {}\n", content, final_response_trimmed).as_bytes()).await;
                    }
                    Err(e) => {
                        eprintln!("Failed to open session file: {}", e);
                    }
                }
            } else {
                let _ = channel_id.say(&http, "Gemini finished its task, but no response was generated. 😶").await;
            }

            if is_first_message {
                let summary = content.chars().take(30).collect::<String>().trim().to_string();
                if let Ok(file_content) = fs::read_to_string(&session_path).await {
                    let updated_content = file_content.replace("# New Session", &format!("# {}", summary));
                    let _ = fs::write(&session_path, updated_content).await;
                }
            }
            if let Some(m) = msg {
                let _ = m.delete_reaction_emoji(&http, '👀').await;
                let _ = m.react(&http, '✅').await;
            }

            // Autonomous Trigger Detection
            let re = Regex::new(r"\[\[trigger:(?P<id>[^\]]+)\]\]").unwrap();
            if let Some(caps) = re.captures(final_response_trimmed) {
                let task_id = &caps["id"];
                println!("Detected autonomous trigger: {}", task_id);
                
                let tasks_json = fs::read_to_string("workspace/tasks.json").await.unwrap_or_else(|_| "{\"tasks\": []}".to_string());
                let task_list: crate::types::TaskList = serde_json::from_str(&tasks_json).unwrap_or(crate::types::TaskList { tasks: vec![] });
                let found_task = task_list.tasks.iter().find(|t| t.id == task_id).cloned();

                if let Some(task) = found_task {
                    // If it has an interval, schedule it if not already scheduled
                    if let Some(interval) = task.interval {
                        let mut scheduled = scheduled_tasks.lock().await;
                        if !scheduled.iter().any(|s| s.task_id == *task_id && s.channel_id == channel_id) {
                            scheduled.push(crate::types::ScheduledTask {
                                task_id: task.id.clone(),
                                channel_id,
                                session_path: session_path.clone(),
                                workspace_path: req.workspace_path.clone(),
                                last_run: chrono::Utc::now(),
                            });
                            println!("AI scheduled task `{}` every {} seconds.", task_id, interval);
                        }
                    } else {
                        let next_request = GeminiRequest {
                            http: http.clone(),
                            channel_id,
                            user_name: "System".to_string(),
                            msg: None,
                            session_path: session_path.clone(),
                            soul_path: req.soul_path.clone(),
                            workspace_path: req.workspace_path.clone(),
                            content: task.prompt,
                            is_first_message: false,
                            attachment_paths: vec![],
                            is_indexing: false,
                        };
                        
                        if queue_size.load(Ordering::SeqCst) < 3 {
                            queue_size.fetch_add(1, Ordering::SeqCst);
                            if let Err(e) = queue_tx.send(next_request).await {
                                eprintln!("Failed to send autonomous trigger to queue: {}", e);
                                queue_size.fetch_sub(1, Ordering::SeqCst);
                            }
                        } else {
                            println!("Autonomous trigger `{}` skipped because queue is full.", task_id);
                            let _ = channel_id.say(&http, format!("Autonomous trigger `{}` skipped because I'm too busy! ⏳", task_id)).await;
                        }
                    }
                } else {
                    let _ = channel_id.say(&http, format!("Error: Autonomous trigger ID `{}` not found.", task_id)).await;
                }
            }

            // AI Download Logic: [[download:filename]]
            let download_re = Regex::new(r"\[\[download:(?P<file>[^\]]+)\]\]").unwrap();
            for caps in download_re.captures_iter(final_response_trimmed) {
                let filename = &caps["file"];
                if let Ok(channel_name) = channel_id.name(&http).await {
                    let sanitized = crate::utils::sanitize_filename(&channel_name);
                    let file_path = format!("workspace/channels/{}/bank/{}", sanitized, filename);
                    
                    if fs::metadata(&file_path).await.is_ok() {
                        match CreateAttachment::path(&file_path).await {
                            Ok(attachment) => {
                                if let Err(e) = channel_id.send_files(&http, vec![attachment], CreateMessage::new()).await {
                                    eprintln!("AI Download: Failed to send file {}: {}", filename, e);
                                }
                            }
                            Err(e) => {
                                eprintln!("AI Download: Failed to create attachment for {}: {}", filename, e);
                            }
                        }
                    } else {
                        let _ = channel_id.say(&http, format!("AI tried to send `{}`, but it's not in the bank. ❌", filename)).await;
                    }
                }
            }

            // Link Summary Logic: [[link_summary: ... ]]
            let link_summary_re = Regex::new(r"\[\[link_summary:(?P<summary>[^\]]+)\]\]").unwrap();
            for caps in link_summary_re.captures_iter(final_response_trimmed) {
                let summary = &caps["summary"];
                if let Ok(channel_name) = channel_id.name(&http).await {
                    let sanitized = crate::utils::sanitize_filename(&channel_name);
                    let link_path = format!("workspace/channels/{}/links.md", sanitized);
                    
                    let mut link_file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&link_path)
                        .await
                        .unwrap();
                    
                    if let Ok(metadata) = fs::metadata(&link_path).await {
                        if metadata.len() == 0 {
                            let _ = link_file.write_all(b"# Channel Link Summaries\n\n").await;
                        }
                    }
                    
                    let summary_entry = format!("- Summary: {}\n\n", summary.trim());
                    let _ = link_file.write_all(summary_entry.as_bytes()).await;
                }
            }
        }
        Ok(s) => {
            if let Some(m) = msg {
                let _ = m.delete_reaction_emoji(&http, '👀').await;
                let _ = m.react(&http, '❌').await;
            }
            let _ = channel_id.say(&http, format!("Gemini CLI exited with failure: {}", s)).await;
            if !final_stderr.is_empty() {
                let _ = channel_id.say(&http, format!("Stderr: ```\n{}\n```", final_stderr)).await;
            }
        }
        Err(e) => {
            if let Some(m) = msg {
                let _ = m.delete_reaction_emoji(&http, '👀').await;
                let _ = m.react(&http, '❌').await;
            }
            let _ = channel_id.say(&http, format!("Process error: {}", e)).await;
        }
    }

    queue_size.fetch_sub(1, Ordering::SeqCst);
}
