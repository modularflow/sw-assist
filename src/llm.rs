use anyhow::{anyhow, bail, Context, Result};
use async_stream::try_stream;
use futures_core::stream::Stream;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{env, pin::Pin};
use std::time::Duration;
use rand::{thread_rng, Rng};

#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

pub enum Provider {
    OpenAi,
}

impl Provider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Provider::OpenAi),
            _ => None,
        }
    }
}

pub struct LlmClient {
    http: Client,
}

impl LlmClient {
    pub fn new_with_timeout(timeout: Duration) -> Result<Self> {
        let http = Client::builder()
            .timeout(timeout)
            .build()?;
        Ok(Self { http })
    }

    pub fn new() -> Result<Self> { Self::new_with_timeout(Duration::from_secs(60)) }

    pub async fn send(&self, provider: Provider, req: LlmRequest) -> Result<LlmResponse> {
        match provider {
            Provider::OpenAi => self.send_openai(req).await,
        }
    }

    pub async fn send_stream(
        &self,
        provider: Provider,
        req: LlmRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        match provider {
            Provider::OpenAi => self.send_openai_stream(req).await,
        }
    }

    async fn send_openai(&self, req: LlmRequest) -> Result<LlmResponse> {
        let base = req
            .api_base
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let url = format!("{}/chat/completions", base);
        // Determine API key requirement based on API base
        let (api_key_opt, require_key): (Option<String>, bool) = if base.contains("api.groq.com") {
            (env::var("GROQ_API_KEY").ok(), true)
        } else if base.contains("127.0.0.1") || base.contains("localhost") {
            (env::var("LMSTUDIO_API_KEY").ok(), false)
        } else {
            (env::var("OPENAI_API_KEY").ok(), true)
        };
        if require_key && api_key_opt.is_none() {
            return Err(anyhow::anyhow!("missing API key for base {}", base)).context("OPENAI_API_KEY not set");
        }

        #[derive(Serialize)]
        struct OpenAiRequest<'a> {
            model: &'a str,
            messages: &'a [ChatMessage],
            stream: bool,
        }

        #[derive(Deserialize)]
        struct OpenAiChoiceDelta {
            content: Option<String>,
        }

        #[derive(Deserialize)]
        struct OpenAiChoiceMessage {
            content: String,
        }

        #[derive(Deserialize)]
        struct OpenAiChoice {
            message: Option<OpenAiChoiceMessage>,
        }

        #[derive(Deserialize)]
        struct OpenAiUsage {
            prompt_tokens: Option<u32>,
            completion_tokens: Option<u32>,
            total_tokens: Option<u32>,
        }

        #[derive(Deserialize)]
        struct OpenAiResponse {
            choices: Vec<OpenAiChoice>,
            usage: Option<OpenAiUsage>,
        }

        let body = OpenAiRequest {
            model: &req.model,
            messages: &req.messages,
            stream: false,
        };

        let res = with_retries(|| async {
            let mut rb = self.http.post(&url).json(&body);
            if let Some(key) = api_key_opt.as_ref() { rb = rb.bearer_auth(key); }
            let resp = rb.send().await?;
            Ok::<_, anyhow::Error>(resp)
        }).await?;
        if res.status() != StatusCode::OK {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            bail!("openai error {}: {}", status, text);
        }
        let parsed: OpenAiResponse = res.json().await?;
        let content = parsed
            .choices
            .get(0)
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let usage = parsed.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        Ok(LlmResponse { content, usage })
    }

    async fn send_openai_stream(
        &self,
        req: LlmRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let base = req
            .api_base
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let url = format!("{}/chat/completions", base);
        // Determine API key requirement based on API base
        let (api_key_opt, require_key): (Option<String>, bool) = if base.contains("api.groq.com") {
            (env::var("GROQ_API_KEY").ok(), true)
        } else if base.contains("127.0.0.1") || base.contains("localhost") {
            (env::var("LMSTUDIO_API_KEY").ok(), false)
        } else {
            (env::var("OPENAI_API_KEY").ok(), true)
        };
        if require_key && api_key_opt.is_none() {
            return Err(anyhow::anyhow!("missing API key for base {}", base)).context("OPENAI_API_KEY not set");
        }

        #[derive(Serialize)]
        struct OpenAiRequest<'a> {
            model: &'a str,
            messages: &'a [ChatMessage],
            stream: bool,
        }

        let body = OpenAiRequest {
            model: &req.model,
            messages: &req.messages,
            stream: true,
        };

        let mut res = with_retries(|| async {
            let mut rb = self.http.post(&url).json(&body);
            if let Some(key) = api_key_opt.as_ref() { rb = rb.bearer_auth(key); }
            let resp = rb.send().await?;
            Ok::<_, anyhow::Error>(resp)
        }).await?;
        if res.status() != StatusCode::OK {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            bail!("openai error {}: {}", status, text);
        }

        // OpenAI streams Server-Sent Events with lines starting with "data: ".
        let byte_stream = res.bytes_stream();
        let s = try_stream! {
            use futures_util::StreamExt;
            let mut content = String::new();
            futures_util::pin_mut!(byte_stream);
            while let Some(chunk) = byte_stream.next().await {
                let bytes = chunk.map_err(|e| anyhow!(e))?;
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    let line = line.trim();
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" { continue; }
                        // Best-effort: extract incremental content field.
                        if let Some(idx) = data.find("\"content\":") {
                            let after = &data[idx + 10..];
                            if let Some(start) = after.find('"') {
                                let after = &after[start + 1..];
                                if let Some(end) = after.find('"') {
                                    let piece = &after[..end];
                                    content.push_str(piece);
                                    yield piece.to_string();
                                }
                            }
                        }
                    }
                }
            }
        };
        Ok(Box::pin(s))
    }
}

pub async fn with_retries<F, Fut, T>(mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, anyhow::Error>>,
{
    let mut attempt = 0u32;
    let max_retries = 3u32;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                attempt += 1;
                if attempt > max_retries {
                    return Err(e).context("request failed after retries");
                }
                let backoff_ms = (2u64.pow(attempt) * 100) + thread_rng().gen_range(0..100);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }
}

// Provider adapter trait + registry
#[async_trait::async_trait]
pub trait ModelProviderAdapter: Send + Sync {
    async fn send(&self, req: LlmRequest) -> Result<LlmResponse>;
    async fn send_stream(
        &self,
        req: LlmRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>>;
}

pub struct OpenAiAdapter {
    client: LlmClient,
}

impl OpenAiAdapter {
    pub fn new_with_timeout(timeout: Duration) -> Result<Self> { Ok(Self { client: LlmClient::new_with_timeout(timeout)? }) }
    pub fn new() -> Result<Self> { Self::new_with_timeout(Duration::from_secs(60)) }
}

#[async_trait::async_trait]
impl ModelProviderAdapter for OpenAiAdapter {
    async fn send(&self, req: LlmRequest) -> Result<LlmResponse> { self.client.send(Provider::OpenAi, req).await }
    async fn send_stream(&self, req: LlmRequest) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> { self.client.send_openai_stream(req).await }
}

use std::collections::HashMap;

pub struct ProviderRegistry {
    map: HashMap<String, Box<dyn ModelProviderAdapter>>, // keyed by provider name (lowercase)
}

impl ProviderRegistry {
    pub fn new() -> Result<Self> {
        Self::new_with_timeout(Duration::from_secs(60))
    }

    pub fn new_with_timeout(timeout: Duration) -> Result<Self> {
        let mut map: HashMap<String, Box<dyn ModelProviderAdapter>> = HashMap::new();
        map.insert("openai".to_string(), Box::new(OpenAiAdapter::new_with_timeout(timeout)?));
        // Placeholder adapters for future providers
        map.insert("anthropic".to_string(), Box::new(NotImplementedAdapter::new("anthropic")));
        map.insert("grok".to_string(), Box::new(NotImplementedAdapter::new("grok")));
        map.insert("xai".to_string(), Box::new(NotImplementedAdapter::new("xai")));
        map.insert("groq".to_string(), Box::new(NotImplementedAdapter::new("groq")));
        map.insert("gemini".to_string(), Box::new(NotImplementedAdapter::new("gemini")));
        map.insert("ollama".to_string(), Box::new(NotImplementedAdapter::new("ollama")));
        map.insert("lmstudio".to_string(), Box::new(NotImplementedAdapter::new("lmstudio")));
        Ok(Self { map })
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn ModelProviderAdapter>> { self.map.get(&name.to_lowercase()) }
}

struct NotImplementedAdapter { name: &'static str }

impl NotImplementedAdapter { fn new(name: &'static str) -> Self { Self { name } } }

#[async_trait::async_trait]
impl ModelProviderAdapter for NotImplementedAdapter {
    async fn send(&self, _req: LlmRequest) -> Result<LlmResponse> { Err(anyhow!("provider '{}' not implemented", self.name)) }
    async fn send_stream(&self, _req: LlmRequest) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> { Err(anyhow!("provider '{}' not implemented", self.name)) }
}

// Minimal credential validation helper used by `sw init`
pub async fn validate_provider_credentials(
    provider: &str,
    api_key_opt: Option<&str>,
    api_base_opt: Option<&str>,
    timeout_secs: Option<u64>,
) -> Result<()> {
    let base = api_base_opt
        .map(|s| s.to_string())
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let key_env = match provider.to_lowercase().as_str() {
        "openai" => "OPENAI_API_KEY",
        "groq" => "GROQ_API_KEY",
        _ => "",
    };
    let key = match api_key_opt {
        Some(k) => k.to_string(),
        None => {
            if key_env.is_empty() { String::new() } else { env::var(key_env).context(format!("{} not set", key_env))? }
        }
    };
    // Skip key requirement for local LM Studio
    let require_key = !(base.contains("127.0.0.1") || base.contains("localhost")) && !key_env.is_empty();
    if require_key && key.trim().is_empty() {
        bail!("missing API key for {}", provider);
    }
    let http = Client::builder().timeout(Duration::from_secs(timeout_secs.unwrap_or(10))).build()?;
    // Use a cheap GET to models endpoint
    let url = format!("{}/models", base);
    let mut rb = http.get(&url);
    if !key.trim().is_empty() { rb = rb.bearer_auth(&key); }
    let resp = rb.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("credential validation failed {}: {}", status, text);
    }
    Ok(())
}


