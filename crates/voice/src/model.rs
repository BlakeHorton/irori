//! The Whisper model file: where it lives in the data dir, and the one-time
//! download from Irori's own GitHub release (blocker b2), checked by
//! SHA-256 while it streams. A partial download is kept as `<file>.part`
//! and replaced from scratch on the next try; the model only appears under
//! its real name once it verified.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::Model;

/// `<data dir>/voice/<model file>`.
pub fn model_path(data_dir: &Path, m: &Model) -> PathBuf {
    data_dir.join("voice").join(m.file)
}

/// Whether the model is there with the right size (the hash was checked
/// when it was downloaded).
pub fn model_ready(data_dir: &Path, m: &Model) -> bool {
    std::fs::metadata(model_path(data_dir, m)).is_ok_and(|md| md.is_file() && md.len() == m.size)
}

/// Downloads `m` from `url` into the data dir and returns its path.
/// `progress(done, total)` is called as bytes arrive. Fails (and leaves no
/// model) on any HTTP error or a size or hash mismatch.
pub async fn download_model(
    data_dir: &Path,
    m: &Model,
    url: &str,
    user_agent: &str,
    progress: &(dyn Fn(u64, u64) + Send + Sync),
) -> Result<PathBuf, String> {
    let dst = model_path(data_dir, m);
    let dir = dst.parent().ok_or("bad model path")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let part = dst.with_extension("bin.part");
    let client = client(user_agent)?;
    let mut resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download failed: HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(m.size);
    let mut f =
        std::fs::File::create(&part).map_err(|e| format!("create {}: {e}", part.display()))?;
    let mut hash = Sha256::new();
    let mut done: u64 = 0;
    progress(0, total);
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("download failed: {e}"))?
    {
        hash.update(&chunk);
        f.write_all(&chunk)
            .map_err(|e| format!("write {}: {e}", part.display()))?;
        done += chunk.len() as u64;
        progress(done, total);
        if done > m.size {
            break;
        }
    }
    f.flush().map_err(|e| e.to_string())?;
    drop(f);
    let got = hex::encode(hash.finalize());
    if done != m.size || got != m.sha256 {
        let _ = std::fs::remove_file(&part);
        return Err(format!(
            "the downloaded model did not verify ({done} bytes, sha256 {got}); try again"
        ));
    }
    std::fs::rename(&part, &dst).map_err(|e| format!("rename to {}: {e}", dst.display()))?;
    Ok(dst)
}

/// The updater's TLS setup: rustls with ring and the platform verifier.
fn client(user_agent: &str) -> Result<reqwest::Client, String> {
    use rustls_platform_verifier::BuilderVerifierExt as _;
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_platform_verifier()
        .map_err(|e| e.to_string())?
        .with_no_client_auth();
    reqwest::Client::builder()
        .tls_backend_preconfigured(tls)
        .user_agent(user_agent)
        .build()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    const BODY: &[u8] = b"not really a whisper model";

    fn fake(sha: &'static str) -> Model {
        Model {
            file: "test-model.bin",
            url: "",
            size: BODY.len() as u64,
            sha256: sha,
        }
    }

    /// Serves BODY once on 127.0.0.1 (loopback only: no firewall prompt).
    async fn serve() -> String {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut s, _) = l.accept().await.unwrap();
            let mut req = [0u8; 1024];
            let _ = s.read(&mut req).await;
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                BODY.len()
            );
            s.write_all(head.as_bytes()).await.unwrap();
            s.write_all(BODY).await.unwrap();
        });
        format!("http://{addr}/m.bin")
    }

    fn sha_of(b: &[u8]) -> String {
        hex::encode(Sha256::digest(b))
    }

    #[tokio::test]
    async fn downloads_and_verifies() {
        let dir = tempfile::tempdir().unwrap();
        let sha: &'static str = Box::leak(sha_of(BODY).into_boxed_str());
        let m = fake(sha);
        assert!(!model_ready(dir.path(), &m));
        let seen = AtomicU64::new(0);
        let url = serve().await;
        let p = download_model(dir.path(), &m, &url, "irori-test", &|d, _| {
            seen.store(d, Ordering::SeqCst)
        })
        .await
        .unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), BODY);
        assert!(model_ready(dir.path(), &m));
        assert_eq!(seen.load(Ordering::SeqCst), BODY.len() as u64);
    }

    #[tokio::test]
    async fn a_bad_hash_leaves_no_model() {
        let dir = tempfile::tempdir().unwrap();
        let m = fake("0000000000000000000000000000000000000000000000000000000000000000");
        let url = serve().await;
        let err = download_model(dir.path(), &m, &url, "irori-test", &|_, _| {})
            .await
            .unwrap_err();
        assert!(err.contains("did not verify"), "{err}");
        assert!(!model_ready(dir.path(), &m));
        assert!(
            !model_path(dir.path(), &m)
                .with_extension("bin.part")
                .exists()
        );
    }
}
