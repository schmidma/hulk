use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

use color_eyre::eyre::{eyre, Result, WrapErr};
use log::{debug, info};

use crate::{
    beacon,
    lola::{Battery, RobotConfiguration},
    service_manager::{ServiceManager, SystemServices},
    termination::TerminationRequest,
};

pub struct Aliveness {
    thread: Option<JoinHandle<Result<()>>>,
}

fn get_hulks_os_version() -> Result<String> {
    let os_release = ini::Ini::load_from_file("/etc/os-release")?;
    Ok(os_release
        .general_section()
        .get("VERSION_ID")
        .expect("Could not query VERSION_ID")
        .to_string())
}

impl Aliveness {
    const BEACON_INTERVAL: Duration = Duration::from_secs(1);
    const BEACON_MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 42);
    const BEACON_PORT: u16 = 4242;

    fn aliveness_thread(
        termination_request: TerminationRequest,
        service_manager: &ServiceManager,
        hulks_os_version: &str,
        hostname: &str,
        robot_configuration: Arc<Mutex<Option<RobotConfiguration>>>,
        battery: Arc<Mutex<Option<Battery>>>,
    ) -> Result<()> {
        info!("Starting beacon service");
        while !termination_request.is_requested() {
            let system_services = SystemServices::query(service_manager)
                .context("Failed to query system service states")?;
            let robot_configuration = robot_configuration
                .lock()
                .unwrap()
                .expect("Expected robot configuration, got None");
            let battery = battery.lock().unwrap().expect("Expected battery, got None");
            beacon::send_beacon_on_all_interfaces(
                Self::BEACON_MULTICAST_GROUP,
                Self::BEACON_PORT,
                &hostname,
                system_services,
                &hulks_os_version,
                robot_configuration,
                battery,
            )?;
            sleep(Self::BEACON_INTERVAL);
        }

        Ok(())
    }

    pub fn start(
        termination_request: TerminationRequest,
        robot_configuration: Arc<Mutex<Option<RobotConfiguration>>>,
        battery: Arc<Mutex<Option<Battery>>>,
    ) -> Result<Self> {
        let hulks_os_version = get_hulks_os_version()?;
        let hostname = hostname::get()
            .wrap_err("failed to query hostname")?
            .to_str()
            .ok_or(eyre!("failed to decode hostname"))?;
        let service_manager = ServiceManager::new()?;
        let thread = spawn(move || {
            while robot_configuration.lock().unwrap().is_none() || battery.lock().unwrap().is_none()
            {
                debug!("Waiting for robot configuration and battery...");
                sleep(Duration::from_secs(1));
            }
            let result = Aliveness::aliveness_thread(
                termination_request.clone(),
                &service_manager,
                &hulks_os_version,
                hostname,
                robot_configuration,
                battery,
            );
            termination_request.terminate();
            result
        });

        debug!("Starting aliveness thread...");
        Ok(Self {
            thread: Some(thread),
        })
    }

    pub fn join(mut self) -> Result<()> {
        self.thread
            .take()
            .unwrap()
            .join()
            .map_err(|error| anyhow!("Failed to join aliveness thread: {:?}", error))?
    }
}
