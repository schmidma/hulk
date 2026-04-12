use std::sync::Arc;

use color_eyre::Result;
use ros_z::{Builder, context::ZContext};
use ros_z_config::prelude::*;
use types::buttons::{ButtonPressType, Buttons};
use types::primary_state::PrimaryState;

use crate::{
    IntoEyreResultExt,
    config::PrimaryStateFilterConfig,
    msgs::IsSafePose,
};

pub async fn run(ctx: Arc<ZContext>) -> Result<()> {
    let node = ctx
        .create_node("primary_state_filter")
        .with_type_description_service()
        .build()
        .into_eyre()?;
    let config = node
        .bind_config_with_metadata_as::<PrimaryStateFilterConfig>("primary_state_filter")
        .into_eyre()?;

    let is_safe_pose_sub = node
        .create_sub::<IsSafePose>("state/is_safe_pose")
        .build()
        .into_eyre()?;
    // TODO: wire real button input once button event mapping exists.
    // let buttons_sub = node
    //     .create_sub::<Buttons<Option<ButtonPressType>>>("state/buttons")
    //     .build()
    //     .into_eyre()?;
    let primary_state_pub = node
        .create_pub::<PrimaryState>("state/primary_state")
        .build()
        .into_eyre()?;

    let mut last_primary_state = PrimaryState::Safe;

    loop {
        let latest_is_safe_pose = is_safe_pose_sub.async_recv().await.into_eyre()?.value;
        let buttons = Buttons {
            f1: None,
            stand: None,
            walking: None,
        };

        let cfg = config.snapshot().typed().clone();

        let next_primary_state = if let Some(injected_primary_state) = cfg.injected_primary_state {
            injected_primary_state
        } else {
            match (last_primary_state, buttons) {
                (
                    _,
                    Buttons {
                        f1: Some(ButtonPressType::Short),
                        ..
                    }
                    | Buttons {
                        stand: Some(ButtonPressType::Short),
                        ..
                    },
                ) => PrimaryState::Safe,
                (
                    PrimaryState::Safe,
                    Buttons {
                        stand: Some(ButtonPressType::Long),
                        ..
                    },
                ) if latest_is_safe_pose => PrimaryState::Initial,
                (
                    PrimaryState::Initial,
                    Buttons {
                        stand: Some(ButtonPressType::Long),
                        ..
                    },
                ) if latest_is_safe_pose => PrimaryState::Playing,
                (PrimaryState::Safe, _) => PrimaryState::Safe,
                _ => last_primary_state,
            }
        };

        last_primary_state = next_primary_state;
        primary_state_pub
            .async_publish(&next_primary_state)
            .await
            .into_eyre()?;
    }
}
