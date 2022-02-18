//! Functionality to get a prepared Spin application configuration from spin.toml.

#![deny(missing_docs)]

/// Module to prepare the assets for the components of an application.
mod assets;
/// Configuration representation for a Spin apoplication as a local spin.toml file.
mod config;

#[cfg(test)]
mod tests;

use anyhow::{anyhow, Context, Result};
use config::{RawAppInformation, RawAppManifest, RawComponentManifest};
use futures::future;
use spin_config::{
    ApplicationInformation, ApplicationOrigin, Configuration, CoreComponent, ModuleSource,
    WasmConfig,
};
use std::path::{Path, PathBuf};
use tokio::{fs::File, io::AsyncReadExt};

/// Given the path to a spin.toml manifest file, prepare its assets locally and
/// get a prepared application configuration consumable by a Spin execution context.
/// If a directory is provided, use it as the base directory to expand the assets,
/// otherwise create a new temporary directory.
pub async fn from_file(
    app: impl AsRef<Path>,
    base_dst: Option<PathBuf>,
) -> Result<Configuration<CoreComponent>> {
    let mut buf = vec![];
    File::open(app.as_ref())
        .await?
        .read_to_end(&mut buf)
        .await
        .with_context(|| anyhow!("Cannot read manifest file from {:?}", app.as_ref()))?;

    let manifest: RawAppManifest = toml::from_slice(&buf)?;

    prepare(manifest, app, base_dst).await
}

async fn prepare(
    raw: RawAppManifest,
    src: impl AsRef<Path>,
    base_dst: Option<PathBuf>,
) -> Result<Configuration<CoreComponent>> {
    let dir = match base_dst {
        Some(d) => d,
        None => tempfile::tempdir()?.into_path(),
    };
    let info = info(raw.info, &src);

    let components = future::join_all(
        raw.components
            .into_iter()
            .map(|c| async { core(c, &src, &dir).await })
            .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .map(|x| x.expect("Cannot prepare component."))
    .collect::<Vec<_>>();

    Ok(Configuration { info, components })
}

/// Given a component manifest, prepare its assets and return a fully formed core component.
async fn core(
    raw: RawComponentManifest,
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
) -> Result<CoreComponent> {
    let src = src
        .as_ref()
        .parent()
        .expect("The application file did not have a parent directory.");
    let source = match raw.source {
        config::RawModuleSource::FileReference(p) => {
            let p = match p.is_absolute() {
                true => p,
                false => src.join(p),
            };

            ModuleSource::FileReference(p)
        }
        config::RawModuleSource::Bindle(_) => {
            todo!("Bindle module sources are not yet supported in file-based app config")
        }
    };

    let id = raw.id;
    let mounts = match raw.wasm.files {
        Some(f) => vec![assets::prepare_component(&f, src, &base_dst, &id).await?],
        None => vec![],
    };
    let environment = raw.wasm.environment.unwrap_or_default();
    let allowed_http_hosts = raw.wasm.allowed_http_hosts.unwrap_or_default();
    let wasm = WasmConfig {
        environment,
        mounts,
        allowed_http_hosts,
    };
    let trigger = raw.trigger;

    Ok(CoreComponent {
        source,
        id,
        wasm,
        trigger,
    })
}

/// Convert the raw application information from the spin.toml manifest to the standard configuration.
fn info(raw: RawAppInformation, src: impl AsRef<Path>) -> ApplicationInformation {
    ApplicationInformation {
        api_version: raw.api_version,
        name: raw.name,
        version: raw.version,
        description: raw.description,
        authors: raw.authors.unwrap_or_default(),
        trigger: raw.trigger,
        namespace: raw.namespace,
        origin: ApplicationOrigin::File(src.as_ref().to_path_buf()),
    }
}