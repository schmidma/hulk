use ros_z::MessageTypeInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, MessageTypeInfo)]
#[ros_msg(type_name = "hulk_ros_z/msg/IsSafePose")]
pub struct IsSafePose {
    pub value: bool,
}

impl ros_z::msg::ZMessage for IsSafePose {
    type Serdes = ros_z::msg::SerdeCdrSerdes<Self>;
}
