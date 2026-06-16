mod capture;
mod config;
mod llm;
mod profiles;
mod keyboard;
mod protocol;

use protocol::{Command, Message};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

fn send_error(tx: &mpsc::Sender<Message>, message: &str) {
    tx.send(Message::Error { message: message.to_string() }).ok();
}

#[tokio::main]
async fn main() {
    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            std::process::exit(1);
        }
    };

    let validation_errors = config.validate();
    if !validation_errors.is_empty() {
        eprintln!("Config validation: {}", validation_errors.join("; "));
        std::process::exit(1);
    }

    let (tx, rx) = mpsc::channel::<Message>();
    std::thread::spawn(move || {
        let stdout = io::stdout();
        let mut writer = io::BufWriter::new(stdout.lock());
        for msg in rx {
            writeln!(writer, "{}", msg.to_ndjson()).ok();
            writer.flush().ok();
        }
    });

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
                send_error(&tx, &format!("Parse error: {}", e));
                continue;
            }
        };

        match cmd {
            Command::Ping => {
                tx.send(Message::Pong).ok();
            }
            Command::Capture => {
                if is_processing.load(Ordering::SeqCst) {
                    send_error(&tx, "Already processing");
                    continue;
                }
                is_processing.store(true, Ordering::SeqCst);

                let system_prompt = match profiles::get_system_prompt(&config.profile) {
                    Some(p) => p,
                    None => {
                        send_error(&tx, &format!("Unknown profile: {}", config.profile));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let image_b64 = match capture::capture_primary_monitor() {
                    Ok(img) => img,
                    Err(e) => {
                        send_error(&tx, &format!("Capture failed: {}", e));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let result = match config.model.as_str() {
                    "gpt-4o" => {
                        llm::stream_openai(&tx, &config.openai_api_key, &system_prompt, &image_b64, None).await
                    }
                    "claude" | "claude-sonnet" => {
                        llm::stream_anthropic(&tx, &config.anthropic_api_key, &system_prompt, &image_b64, None).await
                    }
                    _ => Err(anyhow::anyhow!("Unknown model: {}", config.model)),
                };

                if let Err(e) = result {
                    send_error(&tx, &format!("LLM error: {}", e));
                }

                is_processing.store(false, Ordering::SeqCst);
            }
            Command::Stop => {
                is_processing.store(false, Ordering::SeqCst);
            }
            Command::Shutdown => break,
            Command::StartInputMode => {
                #[cfg(target_os = "windows")]
                {
                    send_error(&tx, "Input mode not yet implemented");
                    tx.send(Message::InputModeState { state: "error".into() }).ok();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    send_error(&tx, "Input mode not supported on this platform");
                    tx.send(Message::InputModeState { state: "error".into() }).ok();
                }
            }
            Command::StopInputMode => {
                tx.send(Message::InputModeState { state: "inactive".into() }).ok();
            }
            Command::CaptureWithText { content } => {
                if is_processing.load(Ordering::SeqCst) {
                    send_error(&tx, "Already processing");
                    continue;
                }
                if content.trim().is_empty() {
                    continue;
                }
                is_processing.store(true, Ordering::SeqCst);

                let system_prompt = match profiles::get_system_prompt(&config.profile) {
                    Some(p) => p,
                    None => {
                        send_error(&tx, &format!("Unknown profile: {}", config.profile));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let image_b64 = match capture::capture_primary_monitor() {
                    Ok(img) => img,
                    Err(e) => {
                        send_error(&tx, &format!("Capture failed: {}", e));
                        is_processing.store(false, Ordering::SeqCst);
                        continue;
                    }
                };

                let result = match config.model.as_str() {
                    "gpt-4o" => {
                        llm::stream_openai(&tx, &config.openai_api_key, &system_prompt, &image_b64, Some(&content)).await
                    }
                    "claude" | "claude-sonnet" => {
                        llm::stream_anthropic(&tx, &config.anthropic_api_key, &system_prompt, &image_b64, Some(&content)).await
                    }
                    _ => Err(anyhow::anyhow!("Unknown model: {}", config.model)),
                };

                if let Err(e) = result {
                    send_error(&tx, &format!("LLM error: {}", e));
                }

                is_processing.store(false, Ordering::SeqCst);
            }
        }
    }
}
