use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use sqlx::SqlitePool;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;

pub struct AppState {
    pub db_pool: SqlitePool,
    pub secret_key: String,
}

pub async fn init_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password TEXT NOT NULL,
            is_admin BOOLEAN NOT NULL DEFAULT FALSE
        ); CREATE TABLE IF NOT EXISTS audio_files (
            id TEXT PRIMARY KEY,
            filename TEXT NOT NULL,
            user_id TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            mime_type TEXT NOT NULL,
            user_folder TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        ); CREATE TABLE IF NOT EXISTS playlists (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            user_id TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        ); CREATE TABLE IF NOT EXISTS playlist_items (
            id TEXT PRIMARY KEY,
            playlist_id TEXT NOT NULL,
            audio_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            FOREIGN KEY (playlist_id) REFERENCES playlists(id),
            FOREIGN KEY (audio_id) REFERENCES audio_files(id)
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub fn load_rustls_config() -> ServerConfig {
    let cert_file =
        &mut BufReader::new(fs::File::open("cert.pem").expect("Cannot open certificate file"));
    let key_file = &mut BufReader::new(fs::File::open("key.pem").expect("Cannot open key file"));

    let cert_chain = certs(cert_file)
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .map(CertificateDer::from)
        .collect::<Vec<_>>();

    let mut keys = pkcs8_private_keys(key_file)
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .map(PrivateKeyDer::from)
        .collect::<Vec<_>>();

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, keys.remove(0))
        .expect("Failed to set up TLS config");

    config
}

pub fn ensure_ssl_cert_exists() -> std::io::Result<()> {
    let cert_path = Path::new("cert.pem");
    let key_path = Path::new("key.pem");

    // Check if certificates already exist
    if cert_path.exists() && key_path.exists() {
        println!("SSL certificates already exist");
        return Ok(());
    }

    println!("Generating SSL certificates...");

    // Create OpenSSL command to generate self-signed certificate
    let output = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:4096",
            "-keyout",
            "key.pem",
            "-out",
            "cert.pem",
            "-days",
            "365",
            "-nodes",
            "-subj",
            "/CN=localhost",
        ])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to generate SSL certificates: {}", error),
        ));
    }

    println!("SSL certificates generated successfully");

    // Set appropriate permissions for key file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(key_path)?.permissions();
        perms.set_mode(0o600); // Read/write for owner only
        fs::set_permissions(key_path, perms)?;
    }

    Ok(())
}
