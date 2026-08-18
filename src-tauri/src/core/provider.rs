// ---------------------------------------------------------------------------
// API Provider + Protocol (OpenAI Responses / Chat Completions) support
// ---------------------------------------------------------------------------
//
// Self-contained provider layer for probing and persisting API providers such
// as OpenAI, DeepSeek and Xiaomi MiMo. MiMo (Token Plan) exposes an
// OpenAI-compatible `/v1/responses` endpoint, so we can detect and adapt it the
// same way we do for any OpenAI-compatible server.
//
// This module deliberately does NOT depend on `crate::core::api_client` so it
// can be built and reasoned about independently. Provider state is persisted as
// JSON under `~/.codex/codexmate/providers.json`.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::platform::paths::CodexPaths;

const PROVIDERS_FILE: &str = "providers.json";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Known first-party provider kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Openai,
    Deepseek,
    Mimo,
    Custom,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Openai => "openai",
            ProviderKind::Deepseek => "deepseek",
            ProviderKind::Mimo => "mimo",
            ProviderKind::Custom => "custom",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(ProviderKind::Openai),
            "deepseek" => Some(ProviderKind::Deepseek),
            "mimo" => Some(ProviderKind::Mimo),
            "custom" => Some(ProviderKind::Custom),
            _ => None,
        }
    }

    /// Default base URL. MiMo defaults to the Token Plan endpoint (`tp-` keys).
    /// Overridable per-call via `customUrl`.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            ProviderKind::Openai => "https://api.openai.com/v1",
            ProviderKind::Deepseek => "https://api.deepseek.com",
            ProviderKind::Mimo => "https://token-plan-cn.xiaomimimo.com/v1",
            ProviderKind::Custom => "",
        }
    }
}

/// Persisted provider configuration (mirrors the TS `ApiProviderConfig`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub provider_type: ProviderKind,
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub supports_responses: bool,
    pub supports_chat_completions: bool,
    #[serde(default)]
    pub model_list: Vec<String>,
    pub default_model: Option<String>,
}

/// On-disk + in-memory store of all configured providers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStore {
    pub providers: Vec<ProviderConfig>,
    pub active_provider: Option<ProviderKind>,
    pub active_model: Option<String>,
}

impl ProviderStore {
    /// Returns a copy with secrets stripped, safe to return to the UI.
    pub fn sanitized(&self) -> ProviderStore {
        let providers = self
            .providers
            .iter()
            .map(|p| {
                let mut c = p.clone();
                c.api_key = None;
                c
            })
            .collect();
        ProviderStore {
            providers,
            active_provider: self.active_provider,
            active_model: self.active_model,
        }
    }
}

/// Per-protocol probe result (mirrors `ApiProtocolTestPayload`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolTestPayload {
    pub provider: String,
    pub protocol: String,
    pub supported: bool,
    pub endpoint: String,
    pub status_code: Option<u16>,
    pub message: String,
}

/// Full provider probe result (mirrors `ApiProviderTestPayload`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTestPayload {
    pub provider: String,
    pub reachable: bool,
    pub supports_responses: bool,
    pub supports_chat_completions: bool,
    pub models_available: bool,
    pub message: String,
    pub protocol_tests: Vec<ProtocolTestPayload>,
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

fn resolve_base_url(kind: ProviderKind, custom_url: &Option<String>) -> String {
    match custom_url {
        Some(u) if !u.trim().is_empty() => u.trim().trim_end_matches('/').to_string(),
        _ => kind.default_base_url().to_string(),
    }
}

fn build_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// MiMo / Token Plan uses the `api-key` header; OpenAI-compatible servers use
/// `Authorization: Bearer`.
fn auth_header(kind: ProviderKind, api_key: &Option<String>) -> Option<(String, String)> {
    match kind {
        ProviderKind::Mimo | ProviderKind::Custom => api_key
            .as_ref()
            .map(|k| ("api-key".to_string(), k.clone())),
        _ => api_key
            .as_ref()
            .map(|k| ("Authorization".to_string(), format!("Bearer {k}"))),
    }
}

fn probe_protocol_inner(
    kind: ProviderKind,
    base_url: &str,
    api_key: &Option<String>,
    protocol: &str,
) -> ProtocolTestPayload {
    let path = match protocol {
        "chat_completions" => "/chat/completions",
        _ => "/responses", // default: treat everything else as the responses protocol
    };
    let endpoint = format!("{base_url}{path}");
    let client = build_client();
    let mut req = client.post(&endpoint);
    if let Some((h, v)) = auth_header(kind, api_key) {
        req = req.header(&h, &v);
    }
    let body = serde_json::json!({
        "model": "ping",
        "input": "ping",
        "max_output_tokens": 1,
    });
    let result = req.json(&body).send();

    match result {
        Ok(resp) => {
            let status = resp.status().as_u16();
            // 404/405/501 => endpoint absent => protocol unsupported.
            let unsupported = status == 404 || status == 405 || status == 501;
            let supported = !unsupported;
            let message = match status {
                200..=299 => "Endpoint responded successfully".to_string(),
                401 | 403 => "Endpoint reachable; authentication required".to_string(),
                422 => "Endpoint reachable (validation error on probe payload)".to_string(),
                400..=499 if unsupported => {
                    "Endpoint not found; protocol likely unsupported".to_string()
                }
                400..=499 => "Endpoint reachable (bad request on probe payload)".to_string(),
                500..=599 => "Server error during probe".to_string(),
                _ => format!("HTTP {status}"),
            };
            ProtocolTestPayload {
                provider: kind.as_str().to_string(),
                protocol: protocol.to_string(),
                supported,
                endpoint,
                status_code: Some(status),
                message,
            }
        }
        Err(e) => ProtocolTestPayload {
            provider: kind.as_str().to_string(),
            protocol: protocol.to_string(),
            supported: false,
            endpoint,
            status_code: None,
            message: format!("Request failed: {e}"),
        },
    }
}

fn list_models_inner(
    kind: ProviderKind,
    base_url: &str,
    api_key: &Option<String>,
) -> Result<Vec<String>, String> {
    let endpoint = format!("{base_url}/models");
    let client = build_client();
    let mut req = client.get(&endpoint);
    if let Some((h, v)) = auth_header(kind, api_key) {
        req = req.header(&h, &v);
    }
    let resp = req.send().map_err(|e| format!("models request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("models endpoint returned HTTP {}", resp.status().as_u16()));
    }
    let json: Value = resp
        .json()
        .map_err(|e| format!("failed to parse models JSON: {e}"))?;
    let models = json
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}

// ---------------------------------------------------------------------------
// Public API (used by commands)
// ---------------------------------------------------------------------------

pub fn probe_provider(
    provider_type: &str,
    custom_url: &Option<String>,
    api_key: &Option<String>,
) -> ProviderTestPayload {
    let kind = ProviderKind::from_str_opt(provider_type).unwrap_or(ProviderKind::Custom);
    let base = resolve_base_url(kind, custom_url.clone());

    if base.is_empty() {
        return ProviderTestPayload {
            provider: provider_type.to_string(),
            reachable: false,
            supports_responses: false,
            supports_chat_completions: false,
            models_available: false,
            message: "No base URL configured for this provider".to_string(),
            protocol_tests: vec![],
        };
    }

    let responses = probe_protocol_inner(kind, &base, api_key, "responses");
    let chat = probe_protocol_inner(kind, &base, api_key, "chat_completions");
    let models_result = list_models_inner(kind, &base, api_key);
    let models_available = models_result.is_ok();

    let reachable = responses.status_code.is_some()
        || chat.status_code.is_some()
        || models_available;

    let message = if reachable {
        format!(
            "Provider reachable. responses={}, chat_completions={}, models={}",
            responses.supported, chat.supported, models_available
        )
    } else {
        "Provider unreachable. Check base URL and network.".to_string()
    };

    ProviderTestPayload {
        provider: provider_type.to_string(),
        reachable,
        supports_responses: responses.supported,
        supports_chat_completions: chat.supported,
        models_available,
        message,
        protocol_tests: vec![responses, chat],
    }
}

pub fn probe_protocol(
    provider_type: &str,
    custom_url: &Option<String>,
    protocol: &str,
    api_key: &Option<String>,
) -> ProtocolTestPayload {
    let kind = ProviderKind::from_str_opt(provider_type).unwrap_or(ProviderKind::Custom);
    let base = resolve_base_url(kind, custom_url.clone());
    if base.is_empty() {
        return ProtocolTestPayload {
            provider: provider_type.to_string(),
            protocol: protocol.to_string(),
            supported: false,
            endpoint: String::new(),
            status_code: None,
            message: "No base URL configured for this provider".to_string(),
        };
    }
    probe_protocol_inner(kind, &base, api_key, protocol)
}

pub fn fetch_models(
    provider_type: &str,
    custom_url: &Option<String>,
    api_key: &Option<String>,
) -> Result<Vec<String>, String> {
    let kind = ProviderKind::from_str_opt(provider_type).unwrap_or(ProviderKind::Custom);
    let base = resolve_base_url(kind, custom_url.clone());
    if base.is_empty() {
        return Err("No base URL configured for this provider".to_string());
    }
    list_models_inner(kind, &base, api_key)
}

// ---------------------------------------------------------------------------
// Persistence (providers.json under codexmate_dir)
// ---------------------------------------------------------------------------

fn providers_path() -> PathBuf {
    let paths = CodexPaths::new();
    paths.codexmate_dir.join(PROVIDERS_FILE)
}

pub fn load_store() -> ProviderStore {
    let p = providers_path();
    match fs::read_to_string(&p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| ProviderStore::default()),
        Err(_) => ProviderStore::default(),
    }
}

fn save_store(store: &ProviderStore) -> Result<(), String> {
    let paths = CodexPaths::new();
    fs::create_dir_all(&paths.codexmate_dir).map_err(|e| e.to_string())?;
    let p = paths.codexmate_dir.join(PROVIDERS_FILE);
    let s = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&p, s).map_err(|e| e.to_string())
}

pub fn list_providers() -> ProviderStore {
    load_store()
}

pub fn upsert_provider(config: ProviderConfig) -> Result<ProviderStore, String> {
    let mut store = load_store();
    let pt = config.provider_type;
    if let Some(existing) = store.providers.iter_mut().find(|p| p.provider_type == pt) {
        *existing = config;
    } else {
        store.providers.push(config);
    }
    save_store(&store)?;
    Ok(store)
}

pub fn set_active(provider_type: &str, model: Option<String>) -> Result<ProviderStore, String> {
    let mut store = load_store();
    let kind = ProviderKind::from_str_opt(provider_type)
        .ok_or_else(|| format!("unknown provider type: {provider_type}"))?;
    if !store.providers.iter().any(|p| p.provider_type == kind) {
        return Err(format!(
            "provider '{provider_type}' is not configured; add it first"
        ));
    }
    store.active_provider = Some(kind);
    store.active_model = model;
    save_store(&store)?;
    Ok(store)
}
