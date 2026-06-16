mod capture;
mod config;
mod llm;
mod profiles;
mod protocol;

use protocol::{Command, Message};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn print_message(msg: &Message) {
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    writeln!(writer, "{}", msg.to_ndjson()).ok();
    writer.flush().ok();
}

fn print_error(message: &str) {
    print_message(&Message::Error {
        message: message.to_string(),
    });
}

#[tokio::main]
async fn main() {
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            print_error(&format!("Config error: {}", e));
            std::process::exit(1);
        }
    };

    let validation_errors = config.validate();
    if !validation_errors.is_empty() {
        print_error(&format!("Config validation: {}", validation_errors.join("; ")));
        std::process::exit(1);
    }

    let is_processing = Arc::new(AtomicBool::new(false));
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let cmd: Command = match serde_json::from_str(trimmed) {
            Ok(c) => c,
            Err(e) => {
                print_error(&format!("Parse error: {}", e));
                continue;
            }
        };

        match cmd {
            Command::Ping => {
                print_message(&Message::Pong);
            }
            Command::Capture => {
                if is_processing.load(Ordering::SeqCst) {
                    print_error("Already processing");
                    continue;
                }
                is_processing.store(true, Ordering::SeqCst);

                let system_prompt = match profiles::get_system_prompt(&config.profile) {
                    Some(p) => p,
                    None => {
                        print_error(&format!("Unknown profile: {}", config.profile));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let image_b64 = match capture::capture_primary_monitor() {
                    Ok(img) => img,
                    Err(e) => {
                        print_error(&format!("Capture failed: {}", e));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let result = match config.model.as_str() {
                    "gpt-4o" => {
                        llm::stream_openai(&config.openai_api_key, &system_prompt, &image_b64).await
                    }
                    "claude" | "claude-sonnet" => {
                        llm::stream_anthropic(&config.anthropic_api_key, &system_prompt, &image_b64).await
                    }
                    _ => Err(anyhow::anyhow!("Unknown model: {}", config.model)),
                };

                if let Err(e) = result {
                    print_error(&format!("LLM error: {}", e));
                }

                is_processing.store(false, Ordering::SeqCst);
            }
            Command::Stop => {
                is_processing.store(false, Ordering::SeqCst);
            }
            Command::Shutdown => break,
            Command::StartInputMode => todo!(),
            Command::StopInputMode => todo!(),
            Command::CaptureWithText { .. } => todo!(),
        }
    }
}
