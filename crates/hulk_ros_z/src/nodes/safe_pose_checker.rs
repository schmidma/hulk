use std::sync::Arc;

use booster::{ImuState, LowState, MotorState};
use color_eyre::Result;
use kinematics::joints::Joints;
use ros_z::{Builder, context::ZContext};
use ros_z_config::prelude::*;

use crate::{IntoEyreResultExt, config::SafePoseCheckerConfig, msgs::IsSafePose};

#[derive(Default)]
pub struct SafePoseChecker {
    last_imu_state: ImuState,
    last_serial_motor_states: Joints<MotorState>,
}

pub async fn run(ctx: Arc<ZContext>) -> Result<()> {
    let node = ctx
        .create_node("safe_pose_checker")
        .with_type_description_service()
        .build()
        .into_eyre()?;
    let config = node
        .bind_config_with_metadata_as::<SafePoseCheckerConfig>("safe_pose_checker")
        .into_eyre()?;
    config
        .add_validation_hook(|cfg: &SafePoseCheckerConfig| {
            for (name, value) in [
                (
                    "safe_pose_checker.joint_position_threshold",
                    cfg.joint_position_threshold,
                ),
                (
                    "safe_pose_checker.joint_velocity_threshold",
                    cfg.joint_velocity_threshold,
                ),
                (
                    "safe_pose_checker.angular_velocity_threshold",
                    cfg.angular_velocity_threshold,
                ),
                (
                    "safe_pose_checker.linear_acceleration_threshold",
                    cfg.linear_acceleration_threshold,
                ),
            ] {
                if !value.is_finite() {
                    return Err(format!("{name} must be finite"));
                }
                if value < 0.0 {
                    return Err(format!("{name} must be >= 0"));
                }
            }

            Ok(())
        })
        .into_eyre()?;

    let low_state_sub = node
        .create_sub::<LowState>("robot_hw/low_state")
        .build()
        .into_eyre()?;
    let is_safe_pose_pub = node
        .create_pub::<IsSafePose>("state/is_safe_pose")
        .build()
        .into_eyre()?;

    let mut checker = SafePoseChecker::default();

    loop {
        let cfg = config.snapshot().typed().clone();
        let low_state = low_state_sub.async_recv().await.into_eyre()?;

        checker.last_imu_state = low_state.imu_state;
        if let Ok(serial_motor_states) = low_state.serial_motor_states() {
            checker.last_serial_motor_states = serial_motor_states;
        }

        let prep_mode_imu_state = cfg.prep_mode_imu_state;
        let prep_mode_serial_motor_states = cfg.prep_mode_serial_motor_states;

        let motor_states_are_safe = motor_states_are_safe(
            &checker.last_serial_motor_states,
            &prep_mode_serial_motor_states,
            cfg.joint_position_threshold,
            cfg.joint_velocity_threshold,
        );
        let imu_state_is_safe = imu_state_is_safe(
            &checker.last_imu_state,
            &prep_mode_imu_state,
            cfg.angular_velocity_threshold,
            cfg.linear_acceleration_threshold,
        );

        is_safe_pose_pub
            .async_publish(&IsSafePose {
                value: motor_states_are_safe && imu_state_is_safe,
            })
            .await
            .into_eyre()?;
    }
}

fn motor_states_are_safe(
    serial_motor_states: &Joints<MotorState>,
    prep_mode_serial_motor_states: &Joints<MotorState>,
    joint_position_threshold: f32,
    joint_velocity_threshold: f32,
) -> bool {
    serial_motor_states
        .into_iter()
        .zip(*prep_mode_serial_motor_states)
        .all(|(current_motor_state, safe_motor_state)| {
            (current_motor_state.position - safe_motor_state.position).abs()
                <= joint_position_threshold
                && (current_motor_state.velocity - safe_motor_state.velocity).abs()
                    <= joint_velocity_threshold
        })
}

fn imu_state_is_safe(
    imu_state: &ImuState,
    prep_mode_imu_state: &ImuState,
    angular_velocity_threshold: f32,
    linear_acceleration_threshold: f32,
) -> bool {
    let angular_velocity_delta = imu_state.angular_velocity - prep_mode_imu_state.angular_velocity;
    let linear_acceleration_delta =
        imu_state.linear_acceleration - prep_mode_imu_state.linear_acceleration;

    angular_velocity_delta.x().abs() <= angular_velocity_threshold
        && angular_velocity_delta.y().abs() <= angular_velocity_threshold
        && angular_velocity_delta.z().abs() <= angular_velocity_threshold
        && linear_acceleration_delta.x().abs() <= linear_acceleration_threshold
        && linear_acceleration_delta.y().abs() <= linear_acceleration_threshold
        && linear_acceleration_delta.z().abs() <= linear_acceleration_threshold
}
