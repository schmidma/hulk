use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::MessageTypeInfo;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    MessageTypeInfo,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
#[ros_msg(type_name = "types/msg/SupportSide")]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Left => Self::Right,
            Side::Right => Self::Left,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct SupportFoot {
    pub support_side: Option<Side>,
    pub changed_this_cycle: bool,
}
