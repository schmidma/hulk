use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::dynamic::{DynamicError, FieldType, MessageSchema};

#[derive(Serialize, Deserialize)]
struct MessageSchemaJson {
    type_name: String,
    package: String,
    name: String,
    fields: Vec<FieldSchemaJson>,
}

#[derive(Serialize, Deserialize)]
struct FieldSchemaJson {
    name: String,
    field_type: FieldTypeJson,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FieldTypeJson {
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float32,
    Float64,
    String,
    BoundedString {
        capacity: usize,
    },
    Message {
        schema: Box<MessageSchemaJson>,
    },
    Optional {
        inner: Box<FieldTypeJson>,
    },
    Enum {
        schema: Box<EnumSchemaJson>,
    },
    Array {
        inner: Box<FieldTypeJson>,
        len: usize,
    },
    Sequence {
        inner: Box<FieldTypeJson>,
    },
    BoundedSequence {
        inner: Box<FieldTypeJson>,
        max: usize,
    },
}

#[derive(Serialize, Deserialize)]
struct EnumSchemaJson {
    type_name: String,
    variants: Vec<EnumVariantSchemaJson>,
}

#[derive(Serialize, Deserialize)]
struct EnumVariantSchemaJson {
    name: String,
    payload: EnumPayloadSchemaJson,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EnumPayloadSchemaJson {
    Unit,
    Newtype { field_type: Box<FieldTypeJson> },
    Tuple { field_types: Vec<FieldTypeJson> },
    Struct { fields: Vec<FieldSchemaJson> },
}

pub fn compute_schema_type_hash(
    schema: &MessageSchema,
) -> Result<ros_z_schema::TypeHash, DynamicError> {
    let schema_json = message_schema_to_json(schema);
    let json = ros_z_schema::to_ros2_json(&schema_json).map_err(|err| {
        DynamicError::SerializationError(format!(
            "failed to serialize schema hash view for {}: {}",
            schema.type_name, err
        ))
    })?;

    let mut hasher = sha2::Sha256::new();
    hasher.update(json.as_bytes());
    Ok(ros_z_schema::TypeHash(hasher.finalize().into()))
}

pub fn schema_to_json(schema: &MessageSchema) -> Result<String, DynamicError> {
    let schema_json = message_schema_to_json(schema);
    serde_json::to_string(&schema_json).map_err(|err| {
        DynamicError::SerializationError(format!(
            "failed to serialize schema for {}: {}",
            schema.type_name, err
        ))
    })
}

pub fn schema_from_json(json: &str) -> Result<Arc<MessageSchema>, DynamicError> {
    let schema_json: MessageSchemaJson = serde_json::from_str(json).map_err(|err| {
        DynamicError::DeserializationError(format!("failed to parse schema JSON: {}", err))
    })?;
    Ok(Arc::new(json_to_message_schema(schema_json)))
}

fn message_schema_to_json(schema: &MessageSchema) -> MessageSchemaJson {
    MessageSchemaJson {
        type_name: schema.type_name.clone(),
        package: schema.package.clone(),
        name: schema.name.clone(),
        fields: schema.fields.iter().map(field_schema_to_json).collect(),
    }
}

fn field_schema_to_json(field: &crate::dynamic::FieldSchema) -> FieldSchemaJson {
    FieldSchemaJson {
        name: field.name.clone(),
        field_type: field_type_to_json(&field.field_type),
    }
}

fn field_type_to_json(field_type: &FieldType) -> FieldTypeJson {
    match field_type {
        FieldType::Bool => FieldTypeJson::Bool,
        FieldType::Int8 => FieldTypeJson::Int8,
        FieldType::Int16 => FieldTypeJson::Int16,
        FieldType::Int32 => FieldTypeJson::Int32,
        FieldType::Int64 => FieldTypeJson::Int64,
        FieldType::Uint8 => FieldTypeJson::Uint8,
        FieldType::Uint16 => FieldTypeJson::Uint16,
        FieldType::Uint32 => FieldTypeJson::Uint32,
        FieldType::Uint64 => FieldTypeJson::Uint64,
        FieldType::Float32 => FieldTypeJson::Float32,
        FieldType::Float64 => FieldTypeJson::Float64,
        FieldType::String => FieldTypeJson::String,
        FieldType::BoundedString(capacity) => FieldTypeJson::BoundedString {
            capacity: *capacity,
        },
        FieldType::Message(schema) => FieldTypeJson::Message {
            schema: Box::new(message_schema_to_json(schema)),
        },
        FieldType::Optional(inner) => FieldTypeJson::Optional {
            inner: Box::new(field_type_to_json(inner)),
        },
        FieldType::Enum(schema) => FieldTypeJson::Enum {
            schema: Box::new(enum_schema_to_json(schema)),
        },
        FieldType::Array(inner, len) => FieldTypeJson::Array {
            inner: Box::new(field_type_to_json(inner)),
            len: *len,
        },
        FieldType::Sequence(inner) => FieldTypeJson::Sequence {
            inner: Box::new(field_type_to_json(inner)),
        },
        FieldType::BoundedSequence(inner, max) => FieldTypeJson::BoundedSequence {
            inner: Box::new(field_type_to_json(inner)),
            max: *max,
        },
    }
}

fn enum_schema_to_json(schema: &crate::dynamic::EnumSchema) -> EnumSchemaJson {
    EnumSchemaJson {
        type_name: schema.type_name.clone(),
        variants: schema.variants.iter().map(enum_variant_to_json).collect(),
    }
}

fn enum_variant_to_json(variant: &crate::dynamic::EnumVariantSchema) -> EnumVariantSchemaJson {
    EnumVariantSchemaJson {
        name: variant.name.clone(),
        payload: enum_payload_to_json(&variant.payload),
    }
}

fn enum_payload_to_json(payload: &crate::dynamic::EnumPayloadSchema) -> EnumPayloadSchemaJson {
    match payload {
        crate::dynamic::EnumPayloadSchema::Unit => EnumPayloadSchemaJson::Unit,
        crate::dynamic::EnumPayloadSchema::Newtype(field_type) => EnumPayloadSchemaJson::Newtype {
            field_type: Box::new(field_type_to_json(field_type)),
        },
        crate::dynamic::EnumPayloadSchema::Tuple(field_types) => EnumPayloadSchemaJson::Tuple {
            field_types: field_types.iter().map(field_type_to_json).collect(),
        },
        crate::dynamic::EnumPayloadSchema::Struct(fields) => EnumPayloadSchemaJson::Struct {
            fields: fields.iter().map(field_schema_to_json).collect(),
        },
    }
}

fn json_to_message_schema(schema: MessageSchemaJson) -> MessageSchema {
    MessageSchema {
        type_name: schema.type_name,
        package: schema.package,
        name: schema.name,
        fields: schema
            .fields
            .into_iter()
            .map(json_to_field_schema)
            .collect(),
        type_hash: None,
    }
}

fn json_to_field_schema(field: FieldSchemaJson) -> crate::dynamic::FieldSchema {
    crate::dynamic::FieldSchema {
        name: field.name,
        field_type: json_to_field_type(field.field_type),
        default_value: None,
    }
}

fn json_to_field_type(field_type: FieldTypeJson) -> FieldType {
    match field_type {
        FieldTypeJson::Bool => FieldType::Bool,
        FieldTypeJson::Int8 => FieldType::Int8,
        FieldTypeJson::Int16 => FieldType::Int16,
        FieldTypeJson::Int32 => FieldType::Int32,
        FieldTypeJson::Int64 => FieldType::Int64,
        FieldTypeJson::Uint8 => FieldType::Uint8,
        FieldTypeJson::Uint16 => FieldType::Uint16,
        FieldTypeJson::Uint32 => FieldType::Uint32,
        FieldTypeJson::Uint64 => FieldType::Uint64,
        FieldTypeJson::Float32 => FieldType::Float32,
        FieldTypeJson::Float64 => FieldType::Float64,
        FieldTypeJson::String => FieldType::String,
        FieldTypeJson::BoundedString { capacity } => FieldType::BoundedString(capacity),
        FieldTypeJson::Message { schema } => {
            FieldType::Message(Arc::new(json_to_message_schema(*schema)))
        }
        FieldTypeJson::Optional { inner } => {
            FieldType::Optional(Box::new(json_to_field_type(*inner)))
        }
        FieldTypeJson::Enum { schema } => FieldType::Enum(Arc::new(json_to_enum_schema(*schema))),
        FieldTypeJson::Array { inner, len } => {
            FieldType::Array(Box::new(json_to_field_type(*inner)), len)
        }
        FieldTypeJson::Sequence { inner } => {
            FieldType::Sequence(Box::new(json_to_field_type(*inner)))
        }
        FieldTypeJson::BoundedSequence { inner, max } => {
            FieldType::BoundedSequence(Box::new(json_to_field_type(*inner)), max)
        }
    }
}

fn json_to_enum_schema(schema: EnumSchemaJson) -> crate::dynamic::EnumSchema {
    crate::dynamic::EnumSchema {
        type_name: schema.type_name,
        variants: schema
            .variants
            .into_iter()
            .map(json_to_enum_variant)
            .collect(),
    }
}

fn json_to_enum_variant(variant: EnumVariantSchemaJson) -> crate::dynamic::EnumVariantSchema {
    crate::dynamic::EnumVariantSchema {
        name: variant.name,
        payload: json_to_enum_payload(variant.payload),
    }
}

fn json_to_enum_payload(payload: EnumPayloadSchemaJson) -> crate::dynamic::EnumPayloadSchema {
    match payload {
        EnumPayloadSchemaJson::Unit => crate::dynamic::EnumPayloadSchema::Unit,
        EnumPayloadSchemaJson::Newtype { field_type } => {
            crate::dynamic::EnumPayloadSchema::Newtype(Box::new(json_to_field_type(*field_type)))
        }
        EnumPayloadSchemaJson::Tuple { field_types } => crate::dynamic::EnumPayloadSchema::Tuple(
            field_types.into_iter().map(json_to_field_type).collect(),
        ),
        EnumPayloadSchemaJson::Struct { fields } => crate::dynamic::EnumPayloadSchema::Struct(
            fields.into_iter().map(json_to_field_schema).collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::{EnumPayloadSchema, EnumSchema, EnumVariantSchema, FieldSchema};

    #[test]
    fn schema_json_roundtrips_extended_schema() {
        let nested = MessageSchema::builder("custom_msgs/msg/Pose2")
            .field("x", FieldType::Float32)
            .field("y", FieldType::Float32)
            .build()
            .unwrap();
        let schema = MessageSchema::builder("custom_msgs/msg/RobotEnvelope")
            .field("label", FieldType::BoundedString(32))
            .field(
                "mission_id",
                FieldType::Optional(Box::new(FieldType::Uint32)),
            )
            .field(
                "state",
                FieldType::Enum(Arc::new(EnumSchema::new(
                    "custom_msgs/msg/RobotState",
                    vec![
                        EnumVariantSchema::new("Idle", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new(
                            "Charging",
                            EnumPayloadSchema::Struct(vec![FieldSchema::new(
                                "minutes_remaining",
                                FieldType::Uint32,
                            )]),
                        ),
                    ],
                ))),
            )
            .field("pose", FieldType::Message(nested))
            .field(
                "waypoints",
                FieldType::Array(Box::new(FieldType::Float64), 3),
            )
            .field("history", FieldType::Sequence(Box::new(FieldType::Int16)))
            .field(
                "samples",
                FieldType::BoundedSequence(Box::new(FieldType::Uint8), 16),
            )
            .build()
            .unwrap();

        let json = schema_to_json(&schema).unwrap();
        let reparsed = schema_from_json(&json).unwrap();
        let reparsed_json = schema_to_json(&reparsed).unwrap();

        assert_eq!(reparsed.type_name, schema.type_name);
        assert_eq!(reparsed.field_count(), schema.field_count());
        assert!(reparsed.uses_extended_types());
        assert_eq!(reparsed_json, json);
    }
}
