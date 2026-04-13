use std::io::Write;
use std::net::SocketAddr;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use tokio::net::TcpListener;

use crate::config::{KnownHostsPolicy, SecretsConfig, SshAuthMethod, SshConfig};

pub struct SshTunnel {
    #[allow(dead_code)] // Held for Drop side-effect: openssh::Session closes the tunnel on drop.
    session: openssh::Session,
    #[allow(dead_code)] // Held so the decrypted-key tempfile lives for the session lifetime.
    decrypted_key: Option<tempfile::NamedTempFile>,
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
        builder.known_hosts_check(match ssh_config.known_hosts {
            KnownHostsPolicy::Strict => openssh::KnownHosts::Strict,
            KnownHostsPolicy::Add => openssh::KnownHosts::Add,
            KnownHostsPolicy::Accept => openssh::KnownHosts::Accept,
        });

        let mut decrypted_key: Option<tempfile::NamedTempFile> = None;

        let mut decrypted_key: Option<tempfile::NamedTempFile> = None;

        match ssh_config.auth_method {
            SshAuthMethod::Key => {
                if let Some(key_path) = &ssh_config.key_path {
                    let keyfile_path = match &secrets.ssh_key_passphrase {
                        Some(passphrase) => {
                            let tmp = decrypt_key_to_tempfile(key_path, passphrase)?;
                            let path = tmp.path().to_path_buf();
                            decrypted_key = Some(tmp);
                            path
                        }
                        None => PathBuf::from(key_path),
                    };
                    builder.keyfile(keyfile_path);
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
        let session = builder.connect(&destination).await.map_err(|e| {
            anyhow::anyhow!(
                "SSH connection failed (host={}:{}, user={}): {e:?}",
                ssh_config.host,
                ssh_config.port,
                ssh_config.username
            )
        })?;

        let local_addr: SocketAddr = format!("127.0.0.1:{local_port}").parse()?;
        let remote_addr: SocketAddr =
            format!("{db_host}:{db_port}")
                .parse()
                .map_err(|e: std::net::AddrParseError| {
                    anyhow::anyhow!("Invalid DB address {db_host}:{db_port}: {e}")
                })?;

        session
            .request_port_forward(openssh::ForwardType::Local, local_addr, remote_addr)
            .await
            .map_err(|e| anyhow::anyhow!("Port forward failed: {e}"))?;

        Ok(Self {
            session,
            decrypted_key,
            local_port,
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }
}

/// Decrypt a passphrase-protected OpenSSH private key in-process and write the
/// unencrypted form to a new tempfile with 0600 permissions. The tempfile is
/// deleted when the returned handle is dropped (i.e. when the tunnel closes).
fn decrypt_key_to_tempfile(
    key_path: &str,
    passphrase: &str,
) -> anyhow::Result<tempfile::NamedTempFile> {
    let encrypted = ssh_key::PrivateKey::read_openssh_file(std::path::Path::new(key_path))
        .map_err(|e| anyhow::anyhow!("Failed to read SSH key at {key_path}: {e}"))?;

    if !encrypted.is_encrypted() {
        // Not actually encrypted — write the original bytes back out to keep
        // the code path uniform. Callers can safely keep using the tempfile.
        let pem = encrypted
            .to_openssh(ssh_key::LineEnding::LF)
            .map_err(|e| anyhow::anyhow!("Failed to serialize SSH key: {e}"))?;
        return write_key_tempfile(pem.as_bytes());
    }

    let decrypted = encrypted
        .decrypt(passphrase.as_bytes())
        .map_err(|_| anyhow::anyhow!("Incorrect SSH key passphrase for {key_path}"))?;
    let pem = decrypted
        .to_openssh(ssh_key::LineEnding::LF)
        .map_err(|e| anyhow::anyhow!("Failed to serialize decrypted SSH key: {e}"))?;
    write_key_tempfile(pem.as_bytes())
}

fn write_key_tempfile(bytes: &[u8]) -> anyhow::Result<tempfile::NamedTempFile> {
    let mut tmp = tempfile::Builder::new()
        .prefix("seeki-ssh-")
        .suffix(".key")
        .tempfile()
        .map_err(|e| anyhow::anyhow!("Failed to create tempfile for decrypted key: {e}"))?;

    #[cfg(unix)]
    {
        let mut perms = tmp.as_file().metadata()?.permissions();
        perms.set_mode(0o600);
        tmp.as_file().set_permissions(perms)?;
    }

    tmp.write_all(bytes)?;
    tmp.flush()?;
    Ok(tmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PASSPHRASE: &str = "correct horse battery staple";

    #[test]
    fn decrypts_encrypted_key_to_usable_tempfile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("id_ed25519");
        let key = ssh_key::PrivateKey::random(
            &mut ssh_key::rand_core::OsRng,
            ssh_key::Algorithm::Ed25519,
        )
        .expect("generate key");
        let encrypted = key
            .encrypt(&mut ssh_key::rand_core::OsRng, TEST_PASSPHRASE)
            .expect("encrypt key");
        encrypted
            .write_openssh_file(&key_path, ssh_key::LineEnding::LF)
            .expect("write encrypted key");

        let tmp = decrypt_key_to_tempfile(key_path.to_str().unwrap(), TEST_PASSPHRASE)
            .expect("decrypt succeeds");

        let loaded = ssh_key::PrivateKey::read_openssh_file(tmp.path()).expect("load tempfile");
        assert!(!loaded.is_encrypted(), "tempfile key must be unencrypted");

        #[cfg(unix)]
        {
            let mode = std::fs::metadata(tmp.path()).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "tempfile must be mode 0600");
        }
    }

    #[test]
    fn wrong_passphrase_surfaces_clear_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("id_ed25519");
        let key = ssh_key::PrivateKey::random(
            &mut ssh_key::rand_core::OsRng,
            ssh_key::Algorithm::Ed25519,
        )
        .expect("generate key");
        let encrypted = key
            .encrypt(&mut ssh_key::rand_core::OsRng, TEST_PASSPHRASE)
            .expect("encrypt key");
        encrypted
            .write_openssh_file(&key_path, ssh_key::LineEnding::LF)
            .expect("write encrypted key");

        let err = decrypt_key_to_tempfile(key_path.to_str().unwrap(), "wrong-passphrase")
            .expect_err("must reject wrong passphrase");
        let msg = err.to_string();
        assert!(
            msg.contains("Incorrect SSH key passphrase"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn unencrypted_key_is_passed_through() {
        let dir = tempfile::tempdir().expect("tempdir");
        let key_path = dir.path().join("id_ed25519");
        let key = ssh_key::PrivateKey::random(
            &mut ssh_key::rand_core::OsRng,
            ssh_key::Algorithm::Ed25519,
        )
        .expect("generate key");
        key.write_openssh_file(&key_path, ssh_key::LineEnding::LF)
            .expect("write unencrypted key");

        let tmp = decrypt_key_to_tempfile(key_path.to_str().unwrap(), "anything-goes")
            .expect("pass-through succeeds");
        let loaded = ssh_key::PrivateKey::read_openssh_file(tmp.path()).expect("load tempfile");
        assert!(!loaded.is_encrypted());
    }
}
