//! Configuration examples from the OpenCode v2.2.0 tagged schemas (#1737).
use agnix_core::{LintConfig, ValidatorRegistry, validate_content};
use std::path::Path;

#[test]
fn opencode_220_model_types_cache_ttl_and_media_storage_remain_accepted() {
    let config = LintConfig::builder()
        .tool_versions(agnix_core::config::ToolVersions {
            opencode: Some("2.2.0".into()),
            ..Default::default()
        })
        .build()
        .unwrap();
    // Model.Type is open-ended in the tagged model.ts, not a closed enum.
    for model_type in ["chat", "image", "decision", "future-type"] {
        for file in ["opencode.json", "opencode.jsonc"] {
            let content = serde_json::json!({
                "experimental": {"media_blob": true},
                "providers": {"example": {
                    "settings": {"cache": {"ttl": "1h"}},
                    "models": {"custom": {
                        "type": model_type,
                        "settings": {"cache": {"ttl": "30m"}}
                    }}
                }}
            });
            let diagnostics = validate_content(
                Path::new(file),
                &content.to_string(),
                &config,
                &ValidatorRegistry::with_defaults(),
            );
            assert!(
                diagnostics.is_empty(),
                "{file}, {model_type}: {diagnostics:?}"
            );
        }
    }
}
