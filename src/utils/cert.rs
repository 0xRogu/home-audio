use std::fs;
use std::path::Path;
use std::process::Command;

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
