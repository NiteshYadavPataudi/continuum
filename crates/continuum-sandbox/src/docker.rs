use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use bollard::container::{
    Config, DownloadFromContainerOptions, LogOutput, UploadToContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CommitContainerOptions;
use bollard::Docker;
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use tokio::sync::Mutex;

use continuum_core::caps::{Cap, HostExec};
use continuum_core::ids::{SandboxId, SnapshotId};
use continuum_core::sandbox::{
    ExecEvent, ExecRequest, ExecStream, Sandbox, SandboxError, SandboxHandle, SandboxSpec,
};

/// Docker-backed sandbox factory.
#[derive(Debug, Clone)]
pub struct DockerSandbox {
    client: Docker,
}

impl DockerSandbox {
    /// Connect to the local Docker daemon.
    pub fn new() -> Result<Self, SandboxError> {
        let client = Docker::connect_with_local_defaults()
            .map_err(|e| SandboxError::BackendUnavailable(e.to_string()))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Sandbox for DockerSandbox {
    type Handle = DockerHandle;

    async fn spawn(&self, spec: SandboxSpec) -> Result<Self::Handle, SandboxError> {
        let image = &spec.image;
        let container_name = format!("continuum-{}", SandboxId::new());

        let config = Config {
            image: Some(image.clone()),
            working_dir: Some("/workspace".into()),
            host_config: Some(bollard::models::HostConfig {
                binds: Some(vec![format!("{}:/workspace", spec.workspace.display())]),
                cpu_quota: Some((spec.cpu_quota * 100_000.0) as i64),
                memory: Some(spec.memory_mb as i64 * 1024 * 1024),
                network_mode: if spec.network_egress {
                    None
                } else {
                    Some("none".into())
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        let container = self
            .client
            .create_container(
                Some(bollard::container::CreateContainerOptions {
                    name: container_name,
                    platform: None,
                }),
                config,
            )
            .await
            .map_err(|e| SandboxError::Image(format!("create container: {e}")))?;

        self.client
            .start_container::<String>(&container.id, None)
            .await
            .map_err(|e| SandboxError::Other(format!("start container: {e}")))?;

        Ok(DockerHandle {
            id: SandboxId::new(),
            container_id: container.id,
            client: self.client.clone(),
            snapshots: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

/// Handle to a running Docker container sandbox.
#[derive(Debug, Clone)]
pub struct DockerHandle {
    id: SandboxId,
    container_id: String,
    client: Docker,
    snapshots: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl SandboxHandle for DockerHandle {
    fn id(&self) -> SandboxId {
        self.id
    }

    async fn exec(
        &self,
        _cap: &Cap<HostExec>,
        cmd: ExecRequest,
    ) -> Result<ExecStream, SandboxError> {
        let exec = self
            .client
            .create_exec(
                &self.container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(cmd.argv),
                    working_dir: cmd.cwd.as_ref().map(|p| p.display().to_string()),
                    env: Some(cmd.env.iter().map(|(k, v)| format!("{k}={v}")).collect()),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| SandboxError::Other(format!("create exec: {e}")))?;

        let output = self
            .client
            .start_exec(&exec.id, None)
            .await
            .map_err(|e| SandboxError::Other(format!("start exec: {e}")))?;

        let stream: BoxStream<'static, Result<ExecEvent, SandboxError>> = match output {
            StartExecResults::Attached { output, .. } => {
                let exec_id = exec.id.clone();
                let client = self.client.clone();
                output
                    .map(|item| match item {
                        Ok(LogOutput::StdOut { message }) => {
                            Ok(ExecEvent::Stdout(message.to_vec()))
                        }
                        Ok(LogOutput::StdErr { message }) => {
                            Ok(ExecEvent::Stderr(message.to_vec()))
                        }
                        Ok(LogOutput::Console { message }) => {
                            Ok(ExecEvent::Stdout(message.to_vec()))
                        }
                        Ok(LogOutput::StdIn { .. }) => Ok(ExecEvent::Stdout(Vec::new())),
                        Err(e) => Err(SandboxError::Other(e.to_string())),
                    })
                    .chain(futures::stream::once(async move {
                        match client.inspect_exec(&exec_id).await {
                            Ok(result) => {
                                let code = result.exit_code.unwrap_or(0) as i32;
                                Ok(ExecEvent::Exit(code))
                            }
                            Err(e) => Err(SandboxError::Other(format!("inspect exec: {e}"))),
                        }
                    }))
                    .boxed()
            }
            StartExecResults::Detached => {
                return Err(SandboxError::Other("exec detached unexpectedly".into()));
            }
        };

        Ok(stream)
    }

    async fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), SandboxError> {
        let mut tar_bytes = Vec::new();
        {
            let mut ar = tar::Builder::new(&mut tar_bytes);
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            ar.append_data(&mut header, path, bytes)
                .map_err(|e| SandboxError::Fs(format!("tar append: {e}")))?;
            ar.finish()
                .map_err(|e| SandboxError::Fs(format!("tar finish: {e}")))?;
        }

        self.client
            .upload_to_container(
                &self.container_id,
                Some(UploadToContainerOptions::<&str> {
                    path: "/",
                    no_overwrite_dir_non_dir: "false",
                }),
                tar_bytes.into(),
            )
            .await
            .map_err(|e| SandboxError::Fs(format!("upload: {e}")))?;

        Ok(())
    }

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, SandboxError> {
        let stream = self.client.download_from_container(
            &self.container_id,
            Some(DownloadFromContainerOptions {
                path: path.display().to_string(),
            }),
        );

        let bytes: Vec<u8> = stream
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| SandboxError::Fs(format!("download: {e}")))?
            .into_iter()
            .flat_map(|b| b.to_vec())
            .collect();

        let mut ar = tar::Archive::new(&bytes[..]);
        if let Some(entry) = ar
            .entries()
            .map_err(|e| SandboxError::Fs(format!("tar entries: {e}")))?
            .next()
        {
            let mut entry = entry.map_err(|e| SandboxError::Fs(format!("tar entry: {e}")))?;
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut data)
                .map_err(|e| SandboxError::Fs(format!("tar read: {e}")))?;
            return Ok(data);
        }

        Err(SandboxError::Fs("empty tar archive".into()))
    }

    async fn snapshot(&self) -> Result<SnapshotId, SandboxError> {
        let snap_id = SnapshotId::new();
        let result = self
            .client
            .commit_container(
                CommitContainerOptions {
                    container: self.container_id.clone(),
                    repo: "continuum-snap".into(),
                    tag: format!("{snap_id}"),
                    comment: "".into(),
                    author: "continuum".into(),
                    pause: true,
                    changes: None::<String>,
                },
                Config::<String> {
                    ..Default::default()
                },
            )
            .await;

        match result {
            Ok(snap) => {
                let mut list = self.snapshots.lock().await;
                list.push(snap.id.unwrap_or_default());
                Ok(snap_id)
            }
            Err(e) => Err(SandboxError::Other(format!("commit: {e}")))?,
        }
    }

    async fn restore(&self, snap: SnapshotId) -> Result<(), SandboxError> {
        self.client
            .stop_container(&self.container_id, None)
            .await
            .map_err(|e| SandboxError::Other(format!("stop container for restore: {e}")))?;

        self.client
            .remove_container(&self.container_id, None)
            .await
            .map_err(|e| SandboxError::Other(format!("remove container for restore: {e}")))?;

        let image = format!("continuum-snap:{snap}");
        let new_name = format!("continuum-{}", SandboxId::new());

        let config = Config {
            image: Some(image),
            host_config: Some(bollard::models::HostConfig {
                ..Default::default()
            }),
            ..Default::default()
        };

        let container = self
            .client
            .create_container(
                Some(bollard::container::CreateContainerOptions {
                    name: new_name,
                    platform: None,
                }),
                config,
            )
            .await
            .map_err(|e| SandboxError::Other(format!("create container from snapshot: {e}")))?;

        self.client
            .start_container::<String>(&container.id, None)
            .await
            .map_err(|e| SandboxError::Other(format!("start container from snapshot: {e}")))?;

        Ok(())
    }

    async fn shutdown(self: Box<Self>) -> Result<(), SandboxError> {
        self.client
            .stop_container(&self.container_id, None)
            .await
            .map_err(|e| SandboxError::Other(format!("stop container: {e}")))?;

        self.client
            .remove_container(&self.container_id, None)
            .await
            .map_err(|e| SandboxError::Other(format!("remove container: {e}")))?;

        Ok(())
    }
}
