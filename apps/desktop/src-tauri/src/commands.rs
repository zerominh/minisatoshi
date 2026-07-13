use descriptor_engine::compile_descriptor_from_config;
use policy_engine::PolicyConfig;
use serde::Serialize;

#[derive(Serialize)]
pub struct CompileVaultResponse {
    pub descriptor: String,
    pub policy_string: String,
}

#[tauri::command]
pub fn compile_vault_descriptor(config: PolicyConfig) -> Result<CompileVaultResponse, String> {
    let policy_string =
        policy_engine::compile_abstract_policy_string(&config).map_err(|e| e.to_string())?;
    let descriptor =
        compile_descriptor_from_config(&config).map_err(|e| e.to_string())?;

    Ok(CompileVaultResponse {
        descriptor,
        policy_string,
    })
}
