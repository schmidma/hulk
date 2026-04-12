use ros_z_config::{ConfigFieldMetadata, ConfigMetadata};
use serde::{Deserialize, Serialize};
use types::primary_state::PrimaryState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimaryStateFilterConfig {
    pub injected_primary_state: Option<PrimaryState>,
}

impl ConfigMetadata for PrimaryStateFilterConfig {
    fn config_metadata() -> Vec<ConfigFieldMetadata> {
        vec![field(
            "injected_primary_state",
            std::any::type_name::<Option<PrimaryState>>(),
            "Optional explicit primary-state override.",
        )]
    }
}

fn field(path: &str, type_name: &str, description: &str) -> ConfigFieldMetadata {
    ConfigFieldMetadata {
        path: path.to_owned(),
        type_name: type_name.to_owned(),
        description: description.to_owned(),
        writable: true,
        min: None,
        max: None,
    }
}
