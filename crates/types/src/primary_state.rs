use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::{MessageTypeInfo, TypeHash};
use serde::{Deserialize, Serialize};

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
}

impl ros_z::msg::ZMessage for PrimaryState {
    type Serdes = ros_z::msg::SerdeCdrSerdes<Self>;
}
