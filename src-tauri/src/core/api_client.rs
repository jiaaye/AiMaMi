use reqwest::blocking::Client;
use reqwest::Url;
use std::time::Duration;

use super::models::{ApiProxyConfigPayload, ApiProxyMode, ApiProxyTestPayload, ApiProviderType, ApiProtocolTestPayload, ApiProviderTestPayload};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// API provider types (internal)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiProvider {
    OpenAI,
    DeepSeek,
    MiMo,
    Custom(String),
}

impl ApiProvider {
    pub fn base_url(&self) -> &str {
        match self {
            ApiProvider::OpenAI => "https://api.openai.com",
            ApiProvider::DeepSeek => "https://api.deepseek.com",
            ApiProvider::MiMo => "https://api.mimo.mi.com",
            ApiProvider::Custom(url) => url,
        }
    }

    pub fn responses_endpoint(&self) -> String {
        match self {
            ApiProvider::OpenAI => format!("{}/v1/responses", self.base_url()),
            ApiProvider::DeepSeek => format!("{}/v1/responses", self.base_url()),
            ApiProvider::MiMo => format!("{}/v1/responses", self.base_url()),
            ApiProvider::Custom(url) => format!("{}/v1/responses", url),
        }
    }

    pub fn chat_completions_endpoint(&self) -> String {
        match self {
            ApiProvider::OpenAI => format!("{}/v1/chat/completions", self.base_url()),
            ApiProvider::DeepSeek => format!("{}/v1/chat/completions", self.base_url()),
            ApiProvider::MiMo => format!("{}/v1/chat/completions", self.base_url()),
            ApiProvider::Custom(url) => format!("{}/v1/chat/completions", url),
        }
    }

    pub fn models_endpoint(&self) -> String {
        match self {
            ApiProvider::OpenAI => format!("{}/v1/models", self.base_url()),
            ApiProvider::DeepSeek => format!("{}/v1/models", self.base_url()),
            ApiProvider::MiMo => format!("{}/v1/models", self.base_url()),
            ApiProvider::Custom(url) => format!("{}/v1/models", url),
        }
    }
}

/// Convert ApiProviderType to ApiProvider
pub fn provider_type_to_provider(provider_type: &ApiProviderType, custom_url: Option<&str>) -> ApiProvider {
    match provider_type {
        ApiProviderType::OpenAI => ApiProvider::OpenAI,
        ApiProviderType::DeepSeek => ApiProvider::DeepSeek,
        ApiProviderType::MiMo => ApiProvider::MiMo,
        ApiProviderType::Custom => {
            let url = custom_url.unwrap_or("https://api.example.com");
            ApiProvider::Custom(url.to_string())
        }
    }
}

/// Sanitize proxy configuration
pub fn sanitize_proxy_config(
    config: &ApiProxyConfigPayload,
) -> Result<ApiProxyConfigPayload, String> {
    match config.mode {
        ApiProxyMode::Direct => Ok(ApiProxyConfigPayload {
            mode: ApiProxyMode::Direct,
            url: None,
        }),
        ApiProxyMode::Manual => {
            let url = config
                .url
                .as_deref()
                .ok_or("Manual proxy mode requires a proxy URL")?
                .trim();
            if url.is_empty() {
                return Err("Invalid proxy URL".to_string());
            }
            // Validate URL format
            Url::parse(url).map_err(|_| "Invalid proxy URL format")?;
            Ok(ApiProxyConfigPayload {
                mode: ApiProxyMode::Manual,
                url: Some(url.to_string()),
            })
        }
    }
}

/// Create an HTTP client with proxy configuration
pub fn create_client(config: &ApiProxyConfigPayload) -> Result<Client, String> {
    let mut builder = Client::builder().timeout(DEFAULT_TIMEOUT);

    match config.mode {
        ApiProxyMode::Direct => {
            // Use system proxy or no proxy
        }
        ApiProxyMode::Manual => {
            if let Some(url) = &config.url {
                let proxy = reqwest::Proxy::all(url)
                    .map_err(|e| format!("Failed to create proxy: {}", e))?;
                builder = builder.proxy(proxy);
            }
        }
    }

    builder
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

/// Test API connectivity
pub fn test_api_connectivity(
    config: &ApiProxyConfigPayload,
    api_key: Option<&str>,
) -> ApiProxyTestPayload {
    let client = match create_client(config) {
        Ok(client) => client,
        Err(e) => {
            return ApiProxyTestPayload {
                code: "client_build_failed".to_string(),
                reachable: false,
                status_code: None,
                message: e,
            };
        }
    };

    // Test with a simple request to the API
    let test_url = "https://api.openai.com/v1/models";
    let api_key = api_key.unwrap_or("test-key");

    match client
        .get(test_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
    {
        Ok(response) => {
            let status = response.status().as_u16() as i32;
            let reachable = status == 200 || status == 401; // 401 means API is reachable but key is invalid
            ApiProxyTestPayload {
                code: if reachable { "ok" } else { "error" }.to_string(),
                reachable,
                status_code: Some(status),
                message: if reachable {
                    "API is reachable".to_string()
                } else {
                    format!("API returned status code: {}", status)
                },
            }
        }
        Err(e) => ApiProxyTestPayload {
            code: "network_error".to_string(),
            reachable: false,
            status_code: None,
            message: format!("Network error: {}", e),
        },
    }
}

/// Detect API proxy configuration
pub fn detect_api_proxy_config(api_key: Option<&str>) -> ApiProxyTestPayload {
    test_api_connectivity(
        &ApiProxyConfigPayload {
            mode: ApiProxyMode::Direct,
            url: None,
        },
        api_key,
    )
}

/// Test MiMo API connectivity
pub fn test_mimo_connectivity(api_key: Option<&str>) -> ApiProxyTestPayload {
    let client = match create_client(&ApiProxyConfigPayload {
        mode: ApiProxyMode::Direct,
        url: None,
    }) {
        Ok(client) => client,
        Err(e) => {
            return ApiProxyTestPayload {
                code: "client_build_failed".to_string(),
                reachable: false,
                status_code: None,
                message: e,
            };
        }
    };

    let test_url = "https://api.mimo.mi.com/v1/models";
    let api_key = api_key.unwrap_or("test-key");

    match client
        .get(test_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
    {
        Ok(response) => {
            let status = response.status().as_u16() as i32;
            let reachable = status == 200 || status == 401;
            ApiProxyTestPayload {
                code: if reachable { "ok" } else { "error" }.to_string(),
                reachable,
                status_code: Some(status),
                message: if reachable {
                    "MiMo API is reachable".to_string()
                } else {
                    format!("MiMo API returned status code: {}", status)
                },
            }
        }
        Err(e) => ApiProxyTestPayload {
            code: "network_error".to_string(),
            reachable: false,
            status_code: None,
            message: format!("Network error: {}", e),
        },
    }
}

/// Test protocol support for a given provider
pub fn test_protocol_support(
    provider: &ApiProvider,
    protocol: &str, // "responses" or "chat_completions"
    api_key: Option<&str>,
) -> ApiProtocolTestPayload {
    let client = match create_client(&ApiProxyConfigPayload {
        mode: ApiProxyMode::Direct,
        url: None,
    }) {
        Ok(client) => client,
        Err(e) => {
            return ApiProtocolTestPayload {
                provider: match provider {
                    ApiProvider::OpenAI => ApiProviderType::OpenAI,
                    ApiProvider::DeepSeek => ApiProviderType::DeepSeek,
                    ApiProvider::MiMo => ApiProviderType::MiMo,
                    ApiProvider::Custom(_) => ApiProviderType::Custom,
                },
                protocol: protocol.to_string(),
                supported: false,
                endpoint: String::new(),
                status_code: None,
                message: e,
            };
        }
    };

    let endpoint = match protocol {
        "responses" => provider.responses_endpoint(),
        "chat_completions" => provider.chat_completions_endpoint(),
        _ => {
            return ApiProtocolTestPayload {
                provider: match provider {
                    ApiProvider::OpenAI => ApiProviderType::OpenAI,
                    ApiProvider::DeepSeek => ApiProviderType::DeepSeek,
                    ApiProvider::MiMo => ApiProviderType::MiMo,
                    ApiProvider::Custom(_) => ApiProviderType::Custom,
                },
                protocol: protocol.to_string(),
                supported: false,
                endpoint: String::new(),
                status_code: None,
                message: format!("Unknown protocol: {}", protocol),
            };
        }
    };

    let api_key = api_key.unwrap_or("test-key");

    match client
        .get(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
    {
        Ok(response) => {
            let status = response.status().as_u16() as i32;
            // 405 Method Not Allowed means the endpoint exists but doesn't support GET
            // 401/403 means the endpoint exists but requires valid credentials
            let supported = status == 405 || status == 401 || status == 403 || status == 200;
            ApiProtocolTestPayload {
                provider: match provider {
                    ApiProvider::OpenAI => ApiProviderType::OpenAI,
                    ApiProvider::DeepSeek => ApiProviderType::DeepSeek,
                    ApiProvider::MiMo => ApiProviderType::MiMo,
                    ApiProvider::Custom(_) => ApiProviderType::Custom,
                },
                protocol: protocol.to_string(),
                supported,
                endpoint: endpoint.clone(),
                status_code: Some(status),
                message: if supported {
                    format!("{} supports {} protocol", provider_name(provider), protocol)
                } else {
                    format!(
                        "{} does not appear to support {} protocol (status: {})",
                        provider_name(provider),
                        protocol,
                        status
                    )
                },
            }
        }
        Err(e) => ApiProtocolTestPayload {
            provider: match provider {
                ApiProvider::OpenAI => ApiProviderType::OpenAI,
                ApiProvider::DeepSeek => ApiProviderType::DeepSeek,
                ApiProvider::MiMo => ApiProviderType::MiMo,
                ApiProvider::Custom(_) => ApiProviderType::Custom,
            },
            protocol: protocol.to_string(),
            supported: false,
            endpoint,
            status_code: None,
            message: format!("Network error: {}", e),
        },
    }
}

/// Test comprehensive provider support
pub fn test_provider_support(
    provider_type: &ApiProviderType,
    custom_url: Option<&str>,
    api_key: Option<&str>,
) -> ApiProviderTestPayload {
    let provider = provider_type_to_provider(provider_type, custom_url);
    
    // Test connectivity
    let client = match create_client(&ApiProxyConfigPayload {
        mode: ApiProxyMode::Direct,
        url: None,
    }) {
        Ok(client) => client,
        Err(e) => {
            return ApiProviderTestPayload {
                provider: provider_type.clone(),
                reachable: false,
                supports_responses: false,
                supports_chat_completions: false,
                models_available: false,
                message: e,
                protocol_tests: Vec::new(),
            };
        }
    };

    // Test models endpoint
    let models_endpoint = provider.models_endpoint();
    let api_key = api_key.unwrap_or("test-key");
    
    let reachable = match client
        .get(&models_endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
    {
        Ok(response) => {
            let status = response.status().as_u16() as i32;
            status == 200 || status == 401 || status == 403
        }
        Err(_) => false,
    };

    // Test protocols
    let responses_test = test_protocol_support(&provider, "responses", api_key);
    let chat_completions_test = test_protocol_support(&provider, "chat_completions", api_key);

    let models_available = match client
        .get(&models_endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
    {
        Ok(response) => {
            let status = response.status().as_u16() as i32;
            status == 200
        }
        Err(_) => false,
    };

    ApiProviderTestPayload {
        provider: provider_type.clone(),
        reachable,
        supports_responses: responses_test.supported,
        supports_chat_completions: chat_completions_test.supported,
        models_available,
        message: if reachable {
            format!("{} is reachable", provider_name(&provider))
        } else {
            format!("{} is not reachable", provider_name(&provider))
        },
        protocol_tests: vec![responses_test, chat_completions_test],
    }
}

/// Get human-readable provider name
fn provider_name(provider: &ApiProvider) -> &str {
    match provider {
        ApiProvider::OpenAI => "OpenAI",
        ApiProvider::DeepSeek => "DeepSeek",
        ApiProvider::MiMo => "MiMo",
        ApiProvider::Custom(_) => "Custom API",
    }
}

/// Send a request using the responses protocol
pub fn send_responses_request(
    provider: &ApiProvider,
    api_key: &str,
    model: &str,
    messages: &[serde_json::Value],
    config: &ApiProxyConfigPayload,
) -> Result<serde_json::Value, String> {
    let client = create_client(config)?;
    let endpoint = provider.responses_endpoint();

    let request_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });

    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "API request failed with status: {}",
            response.status()
        ));
    }

    response
        .json::<serde_json::Value>()
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Send a request using the chat completions protocol
pub fn send_chat_completions_request(
    provider: &ApiProvider,
    api_key: &str,
    model: &str,
    messages: &[serde_json::Value],
    config: &ApiProxyConfigPayload,
) -> Result<serde_json::Value, String> {
    let client = create_client(config)?;
    let endpoint = provider.chat_completions_endpoint();

    let request_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });

    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "API request failed with status: {}",
            response.status()
        ));
    }

    response
        .json::<serde_json::Value>()
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Get available models for a provider
pub fn get_available_models(
    provider: &ApiProvider,
    api_key: Option<&str>,
    config: &ApiProxyConfigPayload,
) -> Result<Vec<String>, String> {
    let client = create_client(config)?;
    let endpoint = provider.models_endpoint();
    let api_key = api_key.unwrap_or("test-key");

    let response = client
        .get(&endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch models: {}",
            response.status()
        ));
    }

    let response_json: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models = response_json["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|model| model["id"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}
