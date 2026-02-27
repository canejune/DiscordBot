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

    if !std::path::Path::new("sessions").exists() {
        let _ = std::fs::create_dir("sessions");
    }

    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            process_gemini_request(req, Arc::clone(&worker_queue_size)).await;
        }
    });

    let handler = Handler {
        active_sessions: Mutex::new(HashMap::new()),
        queue_tx: tx,
        queue_size,
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
