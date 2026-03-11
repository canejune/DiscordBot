mod types;
mod utils;
mod session;
mod gemini;
mod handler;

use std::collections::{HashMap, HashSet};
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

    let (tx, mut rx) = mpsc::channel(10);
    let queue_size = Arc::new(AtomicUsize::new(0));
    let worker_queue_size = Arc::clone(&queue_size);

    if !std::path::Path::new("workspace/sessions").exists() {
        let _ = std::fs::create_dir_all("workspace/sessions");
    }

    let mut active_sessions = HashMap::new();
    let mut workspace_folders = HashMap::new();

    if let Ok(state_json) = std::fs::read_to_string("workspace/sessions/state.json") {
        if let Ok(state) = serde_json::from_str::<crate::types::BotState>(&state_json) {
            active_sessions = state.active_sessions;
            workspace_folders = state.workspace_folders;
            println!("Loaded state for {} channels", active_sessions.len());
        }
    }

    let tx_worker = tx.clone();
    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            process_gemini_request(req, Arc::clone(&worker_queue_size), tx_worker.clone()).await;
        }
    });

    let handler = Handler {
        active_sessions: Mutex::new(active_sessions),
        workspace_folders: Mutex::new(workspace_folders),
        queue_tx: tx,
        queue_size,
        waiting_for_restart: Mutex::new(HashSet::new()),
        start_time: chrono::Utc::now(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
