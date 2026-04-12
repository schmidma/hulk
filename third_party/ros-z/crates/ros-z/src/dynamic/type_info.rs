use tracing::warn;

use crate::dynamic::{MessageSchema, MessageSchemaTypeDescription};
use crate::entity::{TypeHash, TypeInfo, TYPE_HASH_NOT_SUPPORTED};
use crate::ros_msg::dds_type_name_to_canonical;

pub(crate) fn ros_type_name_from_dds(dds_name: &str) -> String {
    dds_type_name_to_canonical(dds_name)
}

pub(crate) fn schema_hash(schema: &MessageSchema) -> Option<TypeHash> {
    let hash = if schema.uses_extended_types() {
        crate::schema_json::compute_schema_type_hash(schema)
    } else {
        schema.compute_type_hash()
    };

    match hash {
        Ok(hash) => Some(hash),
        Err(error) => {
            warn!(
                "[NOD] Failed to compute type hash for {}: {}",
                schema.type_name, error
            );
            None
        }
    }
}

pub(crate) fn schema_type_info(schema: &MessageSchema) -> TypeInfo {
    TypeInfo {
        name: schema.type_name.clone(),
        hash: schema_hash(schema),
    }
}

pub(crate) fn schema_type_info_with_hash(
    schema: &MessageSchema,
    discovered_hash: &str,
) -> TypeInfo {
    TypeInfo {
        name: schema.type_name.clone(),
        hash: if discovered_hash == TYPE_HASH_NOT_SUPPORTED {
            None
        } else {
            match TypeHash::from_rihs_string(discovered_hash) {
                Ok(hash) => Some(hash),
                Err(error) => {
                    warn!(
                        "[NOD] Failed to parse discovered type hash for {}: {} ({})",
                        schema.type_name, discovered_hash, error
                    );
                    None
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::{FieldType, MessageSchema};

    #[test]
    fn schema_type_info_uses_canonical_hash_for_extended_schemas() {
        let schema = MessageSchema::builder("custom_msgs/msg/RobotEnvelope")
            .field(
                "mission_id",
                FieldType::Optional(Box::new(FieldType::Uint32)),
            )
            .build()
            .expect("schema");

        let type_info = schema_type_info(&schema);
        let expected = crate::schema_json::compute_schema_type_hash(&schema).expect("hash");

        assert_eq!(type_info.name, "custom_msgs/msg/RobotEnvelope");
        assert_eq!(type_info.hash, Some(expected));
    }
}
