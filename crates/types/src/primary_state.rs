use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::{
    dynamic::{
        EnumPayloadSchema, EnumSchema, EnumVariantSchema, FieldSchema, FieldType, MessageSchema,
    },
    MessageTypeInfo, TypeHash,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    Hash,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum PrimaryState {
    #[default]
    Safe,
    Stop,
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
}

impl MessageTypeInfo for PrimaryState {
    fn type_name() -> &'static str {
        "hulk_ros_z/msg/PrimaryState"
    }

    fn type_hash() -> TypeHash {
        TypeHash::zero()
    }

    fn message_schema() -> Arc<MessageSchema> {
        Arc::new(MessageSchema {
            type_name: Self::type_name().to_string(),
            package: "hulk_ros_z".to_string(),
            name: "PrimaryState".to_string(),
            fields: vec![FieldSchema::new(
                "value",
                FieldType::Enum(Arc::new(EnumSchema::new(
                    Self::type_name(),
                    vec![
                        EnumVariantSchema::new("Safe", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new("Stop", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new("Initial", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new("Ready", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new("Set", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new("Playing", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new("Penalized", EnumPayloadSchema::Unit),
                        EnumVariantSchema::new("Finished", EnumPayloadSchema::Unit),
                    ],
                ))),
            )],
            type_hash: None,
        })
    }
}

impl ros_z::msg::ZMessage for PrimaryState {
    type Serdes = ros_z::msg::SerdeCdrSerdes<Self>;
}
