use anyhow::{Result, bail};
use std::io::Write;

use crate::protocol::Message;

fn print_message(msg: &Message) {
    let stdout = std::io::stdout();
    let mut writer = std::io::BufWriter::new(stdout.lock());
    writeln!(writer, "{}", msg.to_ndjson()).ok();
    writer.flush().ok();
}

pub async fn stream_openai(
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
) -> Result<()> {
    use async_openai::{
        Client,
        config::OpenAIConfig,
        types::chat::{
            ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
            ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
            ChatCompletionRequestUserMessageContentPart,
            ChatCompletionRequestMessageContentPartTextArgs,
            ChatCompletionRequestMessageContentPartImageArgs,
            CreateChatCompletionRequestArgs, ImageDetail, ImageUrlArgs,
        },
    };
    use futures::StreamExt;

    let config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(config);

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .stream(true)
        .messages(vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()?,
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(ChatCompletionRequestUserMessageContent::Array(vec![
                        ChatCompletionRequestUserMessageContentPart::Text(
                            ChatCompletionRequestMessageContentPartTextArgs::default()
                                .text("Analyze this screenshot and provide the answer.")
                                .build()?,
                        ),
                        ChatCompletionRequestUserMessageContentPart::ImageUrl(
                            ChatCompletionRequestMessageContentPartImageArgs::default()
                                .image_url(
                                    ImageUrlArgs::default()
                                        .url(format!("data:image/png;base64,{}", image_base64))
                                        .detail(ImageDetail::High)
                                        .build()?,
                                )
                                .build()?,
                        ),
                    ]))
                    .build()?,
            ),
        ])
        .build()?;

    let mut stream = client.chat().create_stream(request).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                for choice in response.choices {
                    if let Some(content) = choice.delta.content {
                        print_message(&Message::Token { content });
                    }
                }
            }
            Err(e) => {
                print_message(&Message::Error {
                    message: e.to_string(),
                });
                return Err(e.into());
            }
        }
    }

    print_message(&Message::Done);
    Ok(())
}

pub async fn stream_anthropic(
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
) -> Result<()> {
    use reqwest::Client as HttpClient;
    use reqwest_eventsource::{Event, EventSource};
    use futures::StreamExt;

    let client = HttpClient::new();
    let body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 2048,
        "stream": true,
        "system": system_prompt,
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": image_base64
                        }
                    },
                    {
                        "type": "text",
                        "text": "Analyze this screenshot and provide the answer."
                    }
                ]
            }
        ]
    });

    let mut es = EventSource::new(
        client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .body(body.to_string()),
    )?;

    while let Some(event) = es.next().await {
        match event? {
            Event::Open => continue,
            Event::Message(msg) => {
                let parsed: serde_json::Value = serde_json::from_str(&msg.data)?;
                let event_type = parsed["type"].as_str().unwrap_or("");
                match event_type {
                    "content_block_delta" => {
                        if let Some(text) = parsed["delta"]["text"].as_str() {
                            print_message(&Message::Token {
                                content: text.to_string(),
                            });
                        }
                    }
                    "message_stop" => {
                        print_message(&Message::Done);
                        es.close();
                        return Ok(());
                    }
                    "error" => {
                        let err_msg = parsed["error"]["message"]
                            .as_str()
                            .unwrap_or("Unknown Anthropic error");
                        print_message(&Message::Error {
                            message: err_msg.to_string(),
                        });
                        bail!("Anthropic API error: {}", err_msg);
                    }
                    _ => {}
                }
            }
        }
    }

    print_message(&Message::Done);
    Ok(())
}
