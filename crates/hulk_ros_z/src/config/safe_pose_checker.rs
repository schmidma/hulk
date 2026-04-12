use booster::{ImuState, MotorState};
use kinematics::joints::Joints;
use ros_z_config::{ConfigFieldMetadata, ConfigMetadata};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafePoseCheckerConfig {
    pub joint_position_threshold: f32,
    pub joint_velocity_threshold: f32,
    pub angular_velocity_threshold: f32,
    pub linear_acceleration_threshold: f32,
    pub prep_mode_serial_motor_states: Joints<MotorState>,
    pub prep_mode_imu_state: ImuState,
}

impl ConfigMetadata for SafePoseCheckerConfig {
    fn config_metadata() -> Vec<ConfigFieldMetadata> {
        vec![
            field(
                "joint_position_threshold",
                std::any::type_name::<f32>(),
                "Maximum absolute joint position delta to safe pose.",
            ),
            field(
                "joint_velocity_threshold",
                std::any::type_name::<f32>(),
                "Maximum absolute joint velocity delta to safe pose.",
            ),
            field(
                "angular_velocity_threshold",
                std::any::type_name::<f32>(),
                "Maximum absolute IMU angular velocity delta to safe pose.",
            ),
            field(
                "linear_acceleration_threshold",
                std::any::type_name::<f32>(),
                "Maximum absolute IMU linear acceleration delta to safe pose.",
            ),
            field(
                "prep_mode_serial_motor_states",
                std::any::type_name::<Joints<MotorState>>(),
                "Reference serial motor states for safe pose checking.",
            ),
            field(
                "prep_mode_imu_state",
                std::any::type_name::<ImuState>(),
                "Reference IMU state for safe pose checking.",
            ),
        ]
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
