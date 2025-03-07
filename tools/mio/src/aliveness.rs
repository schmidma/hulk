use std::{
    iter::once,
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use aliveness::{query_aliveness, AlivenessState};
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use itertools::chain;

pub struct AlivenessPlugin;

impl Plugin for AlivenessPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PossibleNaoAddresses::new())
            .insert_resource(AliveRobots::default())
            .add_systems(Startup, start_aliveness);
    }
}

#[derive(Resource)]
pub struct PossibleNaoAddresses(pub Vec<Ipv4Addr>);

impl PossibleNaoAddresses {
    fn new() -> Self {
        let nao_range = 21..=41;
        let possible_addresses: Vec<_> = chain!(
            once(Ipv4Addr::LOCALHOST),
            nao_range.clone().map(|id| Ipv4Addr::new(10, 0, 24, id)),
            nao_range.map(|id| Ipv4Addr::new(10, 1, 24, id)),
        )
        .collect();
        Self(possible_addresses)
    }
}

#[derive(Resource, Default)]
pub struct AliveRobots(pub Vec<(IpAddr, AlivenessState)>);

fn start_aliveness(runtime: Res<TokioTasksRuntime>) {
    runtime.spawn_background_task(|mut context| async move {
        loop {
            let maybe_ips = query_aliveness(Duration::from_millis(200), None).await;
            match maybe_ips {
                Ok(ips) => {
                    context
                        .run_on_main_thread(|context| {
                            let mut alive_robots = context.world.resource_mut::<AliveRobots>();
                            alive_robots.0 = ips;
                        })
                        .await;
                }
                Err(error) => {
                    error!("failed to query aliveness: {error}");
                }
            }
        }
    });
}

impl AliveRobots {
    pub fn is_reachable(&self, address: &IpAddr) -> bool {
        self.0.iter().any(|(ip, _)| ip == address)
    }
}
