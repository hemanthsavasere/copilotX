use anyhow::{Result, bail};
use std::sync::mpsc::Sender;

use crate::protocol::Message;

pub async fn stream_openai(
    tx: &Sender<Message>,
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
    user_text: Option<&str>,
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

    let prompt_text = user_text.unwrap_or("Analyze this screenshot and provide the answer.");

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
                                .text(prompt_text)
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
                        tx.send(Message::Token { content }).ok();
                    }
                }
            }
            Err(e) => {
                tx.send(Message::Error {
                    message: e.to_string(),
                }).ok();
                return Err(e.into());
            }
        }
    }

    tx.send(Message::Done).ok();
    Ok(())
}

pub async fn stream_anthropic(
    tx: &Sender<Message>,
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
    user_text: Option<&str>,
) -> Result<()> {
    use reqwest::Client as HttpClient;
    use reqwest_eventsource::{Event, EventSource};
    use futures::StreamExt;

    let prompt_text = user_text.unwrap_or("Analyze this screenshot and provide the answer.");

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
                        "text": prompt_text
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
                            tx.send(Message::Token {
                                content: text.to_string(),
                            }).ok();
                        }
                    }
                    "message_stop" => {
                        tx.send(Message::Done).ok();
                        es.close();
                        return Ok(());
                    }
                    "error" => {
                        let err_msg = parsed["error"]["message"]
                            .as_str()
                            .unwrap_or("Unknown Anthropic error");
                        tx.send(Message::Error {
                            message: err_msg.to_string(),
                        }).ok();
                        bail!("Anthropic API error: {}", err_msg);
                    }
                    _ => {}
                }
            }
        }
    }

    tx.send(Message::Done).ok();
    Ok(())
}
