// ---------------------------------------------------------------------------
// Tauri commands: API provider + protocol (responses / chat_completions) support
// ---------------------------------------------------------------------------

use crate::core::models::CoreEnvelope;
use crate::core::provider::{self, ProviderConfig, ProviderStore, ProviderTestPayload, ProtocolTestPayload};

#[tauri::command]
pub async fn test_mimo_connectivity(
    api_key: Option<String>,
) -> Result<CoreEnvelope<ProviderTestPayload>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let payload = provider::probe_provider("mimo", &None, &api_key);
        Ok(CoreEnvelope::ok(payload))
    })
    .await
    .map_err(|e| format!("Blocking command task failed: {e}"))?
}

#[tauri::command]
pub async fn test_provider_support(
    provider_type: String,
    custom_url: Option<String>,
    api_key: Option<String>,
) -> Result<CoreEnvelope<ProviderTestPayload>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let payload = provider::probe_provider(&provider_type, &custom_url, &api_key);
        Ok(CoreEnvelope::ok(payload))
    })
    .await
    .map_err(|e| format!("Blocking command task failed: {e}"))?
}

#[tauri::command]
pub async fn test_protocol_support(
    provider_type: String,
    custom_url: Option<String>,
    protocol: String,
    api_key: Option<String>,
) -> Result<CoreEnvelope<ProtocolTestPayload>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let payload = provider::probe_protocol(&provider_type, &custom_url, &protocol, &api_key);
        Ok(CoreEnvelope::ok(payload))
    })
    .await
    .map_err(|e| format!("Blocking command task failed: {e}"))?
}

#[tauri::command]
pub async fn get_available_models(
    provider_type: String,
    custom_url: Option<String>,
    api_key: Option<String>,
) -> Result<CoreEnvelope<Vec<String>>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let models = provider::fetch_models(&provider_type, &custom_url, &api_key)
            .map_err(|e| e.to_string())?;
        Ok(CoreEnvelope::ok(models))
    })
    .await
    .map_err(|e| format!("Blocking command task failed: {e}"))?
}

#[tauri::command]
pub fn list_api_providers() -> Result<CoreEnvelope<ProviderStore>, String> {
    let store = provider::list_providers().sanitized();
    Ok(CoreEnvelope::ok(store))
}

#[tauri::command]
pub fn upsert_api_provider(config: ProviderConfig) -> Result<CoreEnvelope<ProviderStore>, String> {
    let store = provider::upsert_provider(config).map_err(|e| e.to_string())?;
    Ok(CoreEnvelope::ok(store.sanitized()))
}

#[tauri::command]
pub fn set_active_api_provider(
    provider_type: String,
    model: Option<String>,
) -> Result<CoreEnvelope<ProviderStore>, String> {
    let store = provider::set_active(&provider_type, model).map_err(|e| e.to_string())?;
    Ok(CoreEnvelope::ok(store.sanitized()))
}
