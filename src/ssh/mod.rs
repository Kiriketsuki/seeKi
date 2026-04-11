use std::net::SocketAddr;

use tokio::net::TcpListener;

use crate::config::{SecretsConfig, SshAuthMethod, SshConfig};

pub struct SshTunnel {
    session: openssh::Session,
    local_port: u16,
}

impl SshTunnel {
    pub async fn connect(
        ssh_config: &SshConfig,
        secrets: &SecretsConfig,
        db_host: &str,
        db_port: u16,
    ) -> anyhow::Result<Self> {
        // Find a free local port by binding to :0, then releasing it.
        // TOCTOU: Brief window before SSH reuses the port, but failure is immediate if taken.
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let local_port = listener.local_addr()?.port();
        drop(listener);

        let mut builder = openssh::SessionBuilder::default();
        builder.known_hosts_check(openssh::KnownHosts::Add);

        match ssh_config.auth_method {
            SshAuthMethod::Key => {
                if let Some(key_path) = &ssh_config.key_path {
                    if secrets.ssh_key_passphrase.is_some() {
                        anyhow::bail!(
                            "Passphrase-protected SSH keys are not supported in server mode. \
                             Please add your key to your SSH agent using: ssh-add {key_path}"
                        );
                    }
                    builder.keyfile(key_path);
                }
            }
            SshAuthMethod::Agent => {
                // Relies on SSH_AUTH_SOCK — no extra config needed.
            }
            SshAuthMethod::Password => {
                anyhow::bail!(
                    "Password-based SSH auth is not supported yet; use a key file or SSH agent"
                );
            }
        }

        let destination = format!(
            "ssh://{}@{}:{}",
            ssh_config.username, ssh_config.host, ssh_config.port
        );
        let session = builder
            .connect(&destination)
            .await
            .map_err(|e| anyhow::anyhow!("SSH connection failed: {e}"))?;

        let local_addr: SocketAddr = format!("127.0.0.1:{local_port}").parse()?;
        let remote_addr: SocketAddr = format!("{db_host}:{db_port}")
            .parse()
            .map_err(|e: std::net::AddrParseError| {
                anyhow::anyhow!("Invalid DB address {db_host}:{db_port}: {e}")
            })?;

        session
            .request_port_forward(openssh::ForwardType::Local, local_addr, remote_addr)
            .await
            .map_err(|e| anyhow::anyhow!("Port forward failed: {e}"))?;

        Ok(Self { session, local_port })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        // openssh::Session's own Drop closes the ControlMaster connection.
        // No explicit action needed here.
    }
}
