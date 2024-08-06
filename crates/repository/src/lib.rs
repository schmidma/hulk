pub mod configuration;
pub mod data_home;
pub mod download;
pub mod find_root;
pub mod hardware_ids;
pub mod image;
pub mod inspect_version;
pub mod sdk;
pub mod symlink;

//    async fn cargo(
//        &self,
//        action: CargoAction,
//        workspace: bool,
//        profile: &str,
//        target: &str,
//        features: Option<Vec<String>>,
//        passthrough_arguments: &[String],
//    ) -> Result<()> {
//        let os_is_not_linux = !cfg!(target_os = "linux");
//        let use_docker = target == "nao" && os_is_not_linux;
//
//        let cargo_command = format!("cargo {action} ")
//            + format!("--profile {profile} ").as_str()
//            + if let Some(features) = features {
//                let features = features.join(",");
//                format!("--features {features} ")
//            } else {
//                String::new()
//            }
//            .as_str()
//            + if workspace {
//                "--workspace --all-features --all-targets ".to_string()
//            } else {
//                let manifest = format!("crates/hulk_{target}/Cargo.toml");
//                let root = if use_docker {
//                    Path::new("/hulk")
//                } else {
//                    &self.root
//                };
//                format!("--manifest-path={} ", root.join(manifest).display())
//            }
//            .as_str()
//            + "-- "
//            + match action {
//                CargoAction::Clippy => "--deny warnings ",
//                _ => "",
//            }
//            + passthrough_arguments.join(" ").as_str();
//
//        println!("Running: {cargo_command}");
//
//        let shell_command = if use_docker {
//            format!(
//                "docker run --volume={}:/hulk --volume={}:/naosdk/sysroots/corei7-64-aldebaran-linux/home/cargo \
//                --rm --interactive --tty ghcr.io/hulks/naosdk:{SDK_VERSION} /bin/bash -c \
//                '. /naosdk/environment-setup-corei7-64-aldebaran-linux && {cargo_command}'",
//                self.root.display(),
//                self.root.join("naosdk/cargo-home").join(SDK_VERSION).display()
//            )
//        } else if target == "nao" {
//            format!(
//                ". {} && {cargo_command}",
//                self.root
//                    .join(format!(
//                        "naosdk/{SDK_VERSION}/environment-setup-corei7-64-aldebaran-linux"
//                    ))
//                    .display()
//            )
//        } else {
//            cargo_command
//        };
//
//        let status = Command::new("sh")
//            .arg("-c")
//            .arg(shell_command)
//            .status()
//            .await
//            .wrap_err("failed to execute cargo command")?;
//
//        if !status.success() {
//            bail!("cargo command exited with {status}");
//        }
//
//        Ok(())
//    }
//
//    pub async fn build(
//        &self,
//        workspace: bool,
//        profile: &str,
//        target: &str,
//        features: Option<Vec<String>>,
//        passthrough_arguments: &[String],
//    ) -> Result<()> {
//        self.cargo(
//            CargoAction::Build,
//            workspace,
//            profile,
//            target,
//            features,
//            passthrough_arguments,
//        )
//        .await
//    }
//
//    pub async fn check(&self, workspace: bool, profile: &str, target: &str) -> Result<()> {
//        self.cargo(CargoAction::Check, workspace, profile, target, None, &[])
//            .await
//    }
//
//    pub async fn clippy(&self, workspace: bool, profile: &str, target: &str) -> Result<()> {
//        self.cargo(CargoAction::Clippy, workspace, profile, target, None, &[])
//            .await
//    }
//
//    pub async fn run(
//        &self,
//        profile: &str,
//        target: &str,
//        features: Option<Vec<String>>,
//        passthrough_arguments: &[String],
//    ) -> Result<()> {
//        self.cargo(
//            CargoAction::Run,
//            false,
//            profile,
//            target,
//            features,
//            passthrough_arguments,
//        )
//        .await
//    }
//
//    pub async fn set_communication(&self, enable: bool) -> Result<()> {
//        let file_contents = read_to_string(self.root.join("etc/parameters/framework.json"))
//            .await
//            .wrap_err("failed to read framework.json")?;
//        let mut hardware_json: Value =
//            from_str(&file_contents).wrap_err("failed to deserialize framework.json")?;
//
//        hardware_json["communication_addresses"] = if enable {
//            Value::String("[::]:1337".to_string())
//        } else {
//            Value::Null
//        };
//        {
//            let file_contents = to_string_pretty(&hardware_json)
//                .wrap_err("failed to serialize framework.json")?
//                + "\n";
//            write(
//                self.root.join("etc/parameters/framework.json"),
//                file_contents.as_bytes(),
//            )
//            .await
//            .wrap_err("failed to write framework.json")?;
//        }
//        Ok(())
//    }
//
//    pub async fn set_player_number(
//        &self,
//        head_id: &str,
//        player_number: PlayerNumber,
//    ) -> Result<()> {
//        let path = "player_number";
//        let parameters = nest_value_at_path(
//            path,
//            to_value(player_number).wrap_err("failed to serialize player number")?,
//        );
//        serialize(
//            &parameters,
//            Scope {
//                location: Location::All,
//                id: Id::Head,
//            },
//            path,
//            self.parameters_root(),
//            &Ids {
//                body_id: "unknown_body_id".to_string(),
//                head_id: head_id.to_string(),
//            },
//        )
//        .wrap_err("failed to serialize parameters directory")
//    }
//
//    pub async fn set_recording_intervals(
//        &self,
//        recording_intervals: HashMap<String, usize>,
//    ) -> Result<()> {
//        let file_contents = read_to_string(self.root.join("etc/parameters/framework.json"))
//            .await
//            .wrap_err("failed to read framework.json")?;
//        let mut hardware_json: Value =
//            from_str(&file_contents).wrap_err("failed to deserialize framework.json")?;
//
//        hardware_json["recording_intervals"] = to_value(recording_intervals)
//            .wrap_err("failed to convert recording intervals to JSON")?;
//        {
//            let file_contents = to_string_pretty(&hardware_json)
//                .wrap_err("failed to serialize framework.json")?
//                + "\n";
//            write(
//                self.root.join("etc/parameters/framework.json"),
//                file_contents.as_bytes(),
//            )
//            .await
//            .wrap_err("failed to write framework.json")?;
//        }
//        Ok(())
//    }
//
//    pub async fn create_upload_directory(&self, profile: &str) -> Result<(TempDir, PathBuf)> {
//        let upload_directory = tempdir().wrap_err("failed to create temporary directory")?;
//        let hulk_directory = upload_directory.path().join("hulk");
//
//        // the target directory is "debug" with --profile dev...
//        let profile_directory = match profile {
//            "dev" => "debug",
//            other => other,
//        };
//
//        create_dir_all(hulk_directory.join("bin"))
//            .await
//            .wrap_err("failed to create directory")?;
//
//        symlink(self.root.join("etc"), hulk_directory.join("etc"))
//            .await
//            .wrap_err("failed to link etc directory")?;
//
//        symlink(
//            self.root.join(format!(
//                "target/x86_64-aldebaran-linux-gnu/{profile_directory}/hulk_nao"
//            )),
//            hulk_directory.join("bin/hulk"),
//        )
//        .await
//        .wrap_err("failed to link executable")?;
//
//        Ok((upload_directory, hulk_directory))
//    }
//
//    pub async fn get_configured_locations(&self) -> Result<BTreeMap<String, Option<String>>> {
//        let results: Vec<_> = [
//            "nao_location",
//            "webots_location",
//            "behavior_simulator_location",
//        ]
//        .into_iter()
//        .map(|target_name| async move {
//            (
//                target_name,
//                read_link(self.parameters_root().join(target_name))
//                    .await
//                    .wrap_err_with(|| format!("failed reading location symlink for {target_name}")),
//            )
//        })
//        .collect::<FuturesUnordered<_>>()
//        .collect()
//        .await;
//
//        results
//            .into_iter()
//            .map(|(target_name, path)| match path {
//                Ok(path) => Ok((
//                    target_name.to_string(),
//                    Some(
//                        path.file_name()
//                            .ok_or_else(|| eyre!("failed to get file name"))?
//                            .to_str()
//                            .ok_or_else(|| eyre!("failed to convert to UTF-8"))?
//                            .to_string(),
//                    ),
//                )),
//                Err(error)
//                    if error.downcast_ref::<io::Error>().unwrap().kind() == ErrorKind::NotFound =>
//                {
//                    Ok((target_name.to_string(), None))
//                }
//                Err(error) => Err(error),
//            })
//            .collect()
//    }
//
//    pub async fn set_location(&self, target: &str, location: &str) -> Result<()> {
//        let target_location = self.parameters_root().join(format!("{target}_location"));
//        let new_location = Path::new(location);
//        let new_location_path = self.parameters_root().join(location);
//        if !try_exists(new_location_path).await? {
//            let location_set = self.list_available_locations().await?;
//            let available_locations: String = intersperse(
//                location_set
//                    .into_iter()
//                    .map(|location| format!("  - {location}")),
//                "\n".to_string(),
//            )
//            .collect();
//            bail!("location {location} does not exist. \navailable locations are:\n{available_locations}");
//        }
//        let _ = remove_file(&target_location).await;
//        symlink(&new_location, &target_location)
//            .await
//            .wrap_err_with(|| {
//                format!("failed creating symlink from {new_location:?} to {target_location:?}, does the location exist?"
//                )
//            })
//    }
//
//    pub async fn list_available_locations(&self) -> Result<BTreeSet<String>> {
//        let parameters_path = self.root.join("etc/parameters");
//        let mut locations = read_dir(parameters_path)
//            .await
//            .wrap_err("failed parameters root")?;
//        let mut results = BTreeSet::new();
//        while let Ok(Some(entry)) = locations.next_entry().await {
//            if entry.path().is_dir() && !entry.path().is_symlink() {
//                results.insert(
//                    entry
//                        .path()
//                        .file_name()
//                        .ok_or_else(|| eyre!("failed getting file name for location"))?
//                        .to_str()
//                        .ok_or_else(|| eyre!("failed to convert to UTF-8"))?
//                        .to_string(),
//                );
//            }
//        }
//        Ok(results)
//    }
//}
//
//#[derive(Debug, Clone, Copy)]
//enum CargoAction {
//    Build,
//    Check,
//    Clippy,
//    Run,
//}
//
//impl Display for CargoAction {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//        write!(
//            f,
//            "{}",
//            match self {
//                CargoAction::Build => "build",
//                CargoAction::Check => "check",
//                CargoAction::Clippy => "clippy",
//                CargoAction::Run => "run",
//            }
//        )
//    }
//}
