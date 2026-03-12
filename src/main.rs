mod types;
mod utils;
mod session;
mod gemini;
mod handler;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::sync::{mpsc, Mutex};
use serenity::prelude::*;
use crate::handler::Handler;
use crate::gemini::process_gemini_request;
use serde_json;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let (tx, mut rx): (mpsc::Sender<crate::types::GeminiRequest>, mpsc::Receiver<crate::types::GeminiRequest>) = mpsc::channel(10);
    let queue_size = Arc::new(AtomicUsize::new(0));
    let worker_queue_size = Arc::clone(&queue_size);

    if !std::path::Path::new("workspace/sessions").exists() {
        let _ = std::fs::create_dir_all("workspace/sessions");
    }

    let mut active_sessions = HashMap::new();
    let mut workspace_folders = HashMap::new();
    let mut scheduled_tasks = Vec::new();

    if let Ok(state_json) = std::fs::read_to_string("workspace/sessions/state.json") {
        if let Ok(state) = serde_json::from_str::<crate::types::BotState>(&state_json) {
            active_sessions = state.active_sessions;
            workspace_folders = state.workspace_folders;
            scheduled_tasks = state.scheduled_tasks;
            println!("Loaded state for {} channels and {} scheduled tasks", active_sessions.len(), scheduled_tasks.len());
        }
    }

    let tx_worker = tx.clone();
    let scheduled_tasks_mutex = Arc::new(Mutex::new(scheduled_tasks));
    let worker_scheduled_tasks = Arc::clone(&scheduled_tasks_mutex);
    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            println!("[DEBUG] Worker received GeminiRequest for channel: {}", req.channel_id);
            process_gemini_request(
                req,
                Arc::clone(&worker_queue_size),
                tx_worker.clone(),
                Arc::clone(&worker_scheduled_tasks)
            ).await;
        }
    });
    let handler = Handler {
        active_sessions: Mutex::new(active_sessions),
        workspace_folders: Mutex::new(workspace_folders),
        scheduled_tasks: Arc::clone(&scheduled_tasks_mutex),
        queue_tx: tx.clone(),
        queue_size: Arc::clone(&queue_size),
        start_time: chrono::Utc::now(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    let tx_scheduler = tx.clone();
    let scheduler_tasks = Arc::clone(&scheduled_tasks_mutex);
    let http = client.http.clone();
    let scheduler_queue_size = Arc::clone(&queue_size);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30)); // Check every 30 seconds
        loop {
            interval.tick().await;
            let mut tasks_to_run = Vec::new();
            {
                let mut tasks = scheduler_tasks.lock().await;
                let tasks_json = std::fs::read_to_string("workspace/tasks.json").unwrap_or_else(|_| "{\"tasks\": []}".to_string());
                let task_list: crate::types::TaskList = serde_json::from_str(&tasks_json).unwrap_or(crate::types::TaskList { tasks: vec![] });
                
                let now = chrono::Utc::now();

                for scheduled in tasks.iter_mut() {
                    if let Some(task_def) = task_list.tasks.iter().find(|t| t.id == scheduled.task_id) {
                        if let Some(interval_secs) = task_def.interval {
                            let next_run = scheduled.last_run + chrono::Duration::seconds(interval_secs as i64);
                            if now >= next_run {
                                // Check if queue is already full before adding to tasks_to_run
                                if scheduler_queue_size.load(std::sync::atomic::Ordering::SeqCst) < 3 {
                                    tasks_to_run.push((scheduled.clone(), task_def.clone()));
                                    scheduled.last_run = now;
                                } else {
                                    println!("Scheduler: Queue is full (3/3), skipping task {} for now.", task_def.id);
                                }
                            }
                        }
                    }
                }
            }

            for (scheduled, task_def) in tasks_to_run {
                println!("[DEBUG] Scheduler: Running task {}", task_def.id);
                scheduler_queue_size.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let req = crate::types::GeminiRequest {
                    http: http.clone(),
                    channel_id: scheduled.channel_id,
                    user_name: "Scheduler".to_string(),
                    msg: None,
                    session_path: scheduled.session_path,
                    soul_path: if std::path::Path::new("workspace/SOUL.md").exists() { Some("workspace/SOUL.md".to_string()) } else { None },
                    workspace_path: scheduled.workspace_path,
                    content: task_def.prompt,
                    is_first_message: false,
                };
                if let Err(e) = tx_scheduler.send(req).await {
                    println!("[DEBUG] Scheduler: Failed to send request: {}", e);
                    eprintln!("Scheduler: Failed to send request: {}", e);
                    scheduler_queue_size.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                } else {
                    println!("[DEBUG] Scheduler: Successfully sent task {} to queue", task_def.id);
                }
            }
        }
    });

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
