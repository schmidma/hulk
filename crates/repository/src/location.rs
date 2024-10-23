use std::{
    collections::{HashMap, HashSet},
    io::ErrorKind,
    path::Path,
};

use color_eyre::{
    eyre::{bail, eyre, Context},
    Result,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use itertools::intersperse;
use tokio::{
    fs::{read_dir, read_link, remove_file, symlink, try_exists},
    io,
};

pub async fn get_configured_locations(
    repository_root: impl AsRef<Path>,
) -> Result<HashMap<String, Option<String>>> {
    let parameters_root = &repository_root.as_ref().join("etc/parameters");
    let results: Vec<_> = [
        "nao_location",
        "webots_location",
        "behavior_simulator_location",
    ]
    .into_iter()
    .map(|target_name| async move {
        (
            target_name,
            read_link(parameters_root.join(target_name))
                .await
                .wrap_err_with(|| format!("failed reading location symlink for {target_name}")),
        )
    })
    .collect::<FuturesUnordered<_>>()
    .collect()
    .await;

    results
        .into_iter()
        .map(|(target_name, path)| match path {
            Ok(path) => Ok((
                target_name.to_string(),
                Some(
                    path.file_name()
                        .ok_or_else(|| eyre!("failed to get file name"))?
                        .to_str()
                        .ok_or_else(|| eyre!("failed to convert to UTF-8"))?
                        .to_string(),
                ),
            )),
            Err(error)
                if error.downcast_ref::<io::Error>().unwrap().kind() == ErrorKind::NotFound =>
            {
                Ok((target_name.to_string(), None))
            }
            Err(error) => Err(error),
        })
        .collect()
}

pub async fn set_location(
    target: &str,
    location: &str,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    let parameters_root = repository_root.as_ref().join("etc/parameters");
    let target_location = parameters_root.join(format!("{target}_location"));
    let new_location = Path::new(location);
    let new_location_path = parameters_root.join(location);
    if !try_exists(new_location_path).await? {
        let location_set = list_available_locations(&repository_root).await?;
        let available_locations: String = intersperse(
            location_set
                .into_iter()
                .map(|location| format!("  - {location}")),
            "\n".to_string(),
        )
        .collect();
        bail!(
            "location {location} does not exist. \navailable locations are:\n{available_locations}"
        );
    }
    let _ = remove_file(&target_location).await;
    symlink(&new_location, &target_location)
            .await
            .wrap_err_with(|| {
                format!("failed creating symlink from {new_location:?} to {target_location:?}, does the location exist?"
                )
            })
}

pub async fn list_available_locations(
    repository_root: impl AsRef<Path>,
) -> Result<HashSet<String>> {
    let parameters_root = repository_root.as_ref().join("etc/parameters");
    let mut locations = read_dir(parameters_root)
        .await
        .wrap_err("failed to read parameters directory")?;
    let mut results = HashSet::new();
    while let Ok(Some(entry)) = locations.next_entry().await {
        if entry.path().is_dir() && !entry.path().is_symlink() {
            results.insert(
                entry
                    .path()
                    .file_name()
                    .ok_or_else(|| eyre!("failed getting file name for location"))?
                    .to_str()
                    .ok_or_else(|| eyre!("failed to convert to UTF-8"))?
                    .to_string(),
            );
        }
    }
    Ok(results)
}
