# opencode-go (OpenCode Zen) LLM Provider Support

**Date:** 2026-06-21
**Status:** Approved (Approach A)
**Scope:** Add the OpenCode Zen provider as a third LLM backend in CopilotX, alongside the existing OpenAI and Anthropic providers.

## Goal

Allow a CopilotX user to configure `model: "kimi-k2.6"` with an `opencodeGoApiKey` and have the Rust sidecar stream answers from the OpenCode Zen API, using the same NDJSON token pipeline the other providers already use.

## Context

CopilotX currently supports two LLM providers, dispatched by string-matching `config.model` in `sidecar/src/main.rs`:

| `model` value        | Dispatch                      |
| -------------------- | ----------------------------- |
| `gpt-4o`             | `llm::stream_openai`          |
| `claude`, `claude-sonnet` | `llm::stream_anthropic` |

Both `stream_openai` and `stream_anthropic` take a captured screenshot (base64 PNG), a system prompt from `profiles.rs`, optional user text, and stream `Message::Token` over an `mpsc::Sender` to the NDJSON stdout writer.

`stream_openai` (in `sidecar/src/llm.rs`) is built on the `async-openai` crate and currently hardcodes two values:

- API base URL: `https://api.openai.com/v1` (the `async-openai` default)
- Model name: `"gpt-4o"` (passed to `.model(...)` in the request builder)

The OpenCode Zen endpoint documented at <https://opencode.ai/docs/zen/> is OpenAI-compatible for several models and uses the `/v1/chat/completions` route:

- Endpoint: `https://opencode.ai/zen/v1/chat/completions`
- SDK package: `@ai-sdk/openai-compatible`
- Request/response shape: OpenAI Chat Completions (SSE streaming)
- Auth: `Authorization: Bearer <opencodeGoApiKey>`

`async-openai`'s `OpenAIConfig` supports `.with_api_base(url)` and `.with_api_key(key)`, so the same `stream_openai` function can serve both OpenAI itself and OpenCode Zen by receiving the base URL and model name as parameters. No new HTTP code or dependency is required.

## Design

### Approach A: parameterize `stream_openai`, add `kimi-k2.6` model value

Reuse the existing `stream_openai` function by adding two parameters: `base_url` and `model`. Add a new `opencodeGoApiKey` config field and a new `kimi-k2.6` entry to the valid-model list. The Zen base URL (`https://opencode.ai/zen/v1`) is a constant used at the dispatch site in `main.rs`.

Rejected alternatives:

- **Approach B (separate `stream_opencode` fn):** duplicates ~80 lines of OpenAI request-building logic for no benefit. The two functions would be identical except for two string values.
- **Approach C (generic `opencodeBaseUrl` + `opencodeModel` config fields):** speculative flexibility not requested; violates the project's Simplicity-First guideline.

### Data Flow

```
config.json (model=kimi-k2.6, opencodeGoApiKey=...)
  -> Electron loads/validates config (config.ts)
  -> sidecar spawns, loads config (config.rs), validates
  -> hotkey -> Capture -> main.rs dispatches on "kimi-k2.6"
  -> llm::stream_openai(
       tx,
       api_key   = config.opencode_go_api_key,
       base_url  = "https://opencode.ai/zen/v1",
       model     = "kimi-k2.6",
       system_prompt, image_b64, user_text)
  -> async-openai Client::with_config(
       OpenAIConfig::new()
         .with_api_key(api_key)
         .with_api_base(base_url))
  -> POST {base_url}/chat/completions (SSE stream)
  -> for each delta: Message::Token { content }
  -> stdout NDJSON -> Electron IPC -> overlay
```

The flow is identical to the existing `gpt-4o` path; only the base URL, model string, and API key source differ.

## Components and Changes

### 1. `sidecar/src/llm.rs`

Change the signature of `stream_openai` from:

```rust
pub async fn stream_openai(
    tx: &Sender<Message>,
    api_key: &str,
    system_prompt: &str,
    image_base64: &str,
    user_text: Option<&str>,
) -> Result<()>
```

to:

```rust
pub async fn stream_openai(
    tx: &Sender<Message>,
    api_key: &str,
    base_url: &str,
    model: &str,
    system_prompt: &str,
    image_base64: &str,
    user_text: Option<&str>,
) -> Result<()>
```

Inside the function:

- Replace `let config = OpenAIConfig::new().with_api_key(api_key);` with `let config = OpenAIConfig::new().with_api_key(api_key).with_api_base(base_url);`
- Replace `.model("gpt-4o")` in the request builder with `.model(model)`.

No other logic changes. The SSE consumption loop, message emission, and error handling stay the same.

### 2. `sidecar/src/config.rs`

Add the new field to `Config`:

```rust
#[serde(default)]
pub opencode_go_api_key: String,
```

Add `"kimi-k2.6"` to the valid-model match in `validate`:

```rust
if !matches!(self.model.as_str(),
    "gpt-4o" | "claude" | "claude-sonnet" | "kimi-k2.6")
{
    errors.push(format!(
        "Unknown model: {}. Supported: gpt-4o, claude, claude-sonnet, kimi-k2.6",
        self.model
    ));
}
```

Add a validation rule:

```rust
if self.model == "kimi-k2.6" && self.opencode_go_api_key.is_empty() {
    errors.push("opencodeGoApiKey is required when model is kimi-k2.6".to_string());
}
```

### 3. `sidecar/src/main.rs`

Both dispatch sites (`Command::Capture` and `Command::CaptureWithText`) gain a `kimi-k2.6` arm and the existing `gpt-4o` arms are updated to pass the new parameters explicitly:

```rust
let result = match config.model.as_str() {
    "gpt-4o" => llm::stream_openai(
        &tx, &config.openai_api_key,
        "https://api.openai.com/v1", "gpt-4o",
        &system_prompt, &image_b64, None,
    ).await,
    "kimi-k2.6" => llm::stream_openai(
        &tx, &config.opencode_go_api_key,
        "https://opencode.ai/zen/v1", "kimi-k2.6",
        &system_prompt, &image_b64, None,
    ).await,
    "claude" | "claude-sonnet" => {
        llm::stream_anthropic(&tx, &config.anthropic_api_key, &system_prompt, &image_b64, None).await
    }
    _ => Err(anyhow::anyhow!("Unknown model: {}", config.model)),
};
```

The `CaptureWithText` site mirrors this with `Some(&content)` as the final argument. The base URLs are string literals at the dispatch site rather than config fields (Approach C was rejected).

### 4. `copilotx/src/main/config.ts`

Add the new field to the `AppConfig` interface:

```ts
opencodeGoApiKey: string
```

Add `"kimi-k2.6"` to the valid-model list:

```ts
if (!['gpt-4o', 'claude', 'claude-sonnet', 'kimi-k2.6'].includes(config.model)) {
  errors.push(`Unknown model: ${config.model}. Supported: gpt-4o, claude, claude-sonnet, kimi-k2.6`)
}
```

Add the validation rule:

```ts
if (config.model === 'kimi-k2.6' && !config.opencodeGoApiKey) {
  errors.push('opencodeGoApiKey is required when model is kimi-k2.6')
}
```

### 5. `copilotx/config/config.json`

Add the new field to the template:

```json
"opencodeGoApiKey": ""
```

## Testing

### `sidecar/src/config.rs` unit tests

- Add a test asserting a config with `model: "kimi-k2.6"` and a non-empty `opencodeGoApiKey` produces zero validation errors.
- Add a test asserting a config with `model: "kimi-k2.6"` and an empty `opencodeGoApiKey` produces an error containing `"opencodeGoApiKey"`.

### `sidecar/tests/integration.rs`

No new integration test. The existing ping/pong coverage exercises config loading; a live network call to Zen is out of scope for unit/integration tests. The `valid_config_json()` helper in this file does not need updating (it uses `gpt-4o`).

### `copilotx/src/main/__tests__/config.test.ts`

- Update the `validConfig` fixture to include `opencodeGoApiKey: ''`.
- Add a test: `{ ...validConfig, model: 'kimi-k2.6', opencodeGoApiKey: 'sk-zen-test', openaiApiKey: '' }` returns zero errors.
- Add a test: `{ ...validConfig, model: 'kimi-k2.6', opencodeGoApiKey: '' }` returns an error containing `"opencodeGoApiKey"`.

### Verification commands

```bash
cd copilotx/sidecar && cargo test && cargo clippy -- -D warnings
cd copilotx && pnpm run test && pnpm run typecheck && pnpm run lint
```

## Backward Compatibility

- `opencode_go_api_key` is `#[serde(default)]` in Rust; missing fields deserialize to `""`. Existing `config.json` files without `opencodeGoApiKey` continue to load.
- The TS `AppConfig` interface gains a required field, but `loadConfig` does a plain `JSON.parse` and existing configs work because validation only triggers on `model === "kimi-k2.6"`. (The template gains the field so new installs have it.)
- The `gpt-4o` and `claude`/`claude-sonnet` paths keep working. Their base URLs become explicit constants at the dispatch site but the runtime behavior is identical to before.
- Config field additions are backward-compatible per the project's config rule: "add fields, never rename".

## Error Handling

`stream_openai`'s existing error path already maps HTTP/parse errors to `Message::Error { message }` over the channel and returns `anyhow::Result`. No new error enum, retry logic, or timeout is introduced. A failed Zen request surfaces to the overlay as an error string exactly as a failed OpenAI request does today.

## Out of Scope

- A generic `opencodeBaseUrl` / `opencodeModel` configurable pair (Approach C).
- Support for the Anthropic-format Zen endpoint (`https://opencode.ai/zen/v1/messages`) — only the OpenAI-compatible `/v1/chat/completions` route is wired.
- Other Zen OpenAI-compatible models (glm-5.1, deepseek-v4-pro, grok-build-0.1, etc.) — only `kimi-k2.6` as requested. Adding more later is a one-line `match` arm and a valid-model list entry.
- README "Supported models" line update beyond adding `kimi-k2.6`.
- Token-usage / billing tracking. Zen bills per request against the user's Zen account balance; CopilotX does not inspect or display usage.
