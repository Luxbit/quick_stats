use std::process::Command;
use std::time::Instant;
use std::time::Duration;

use reqwest::Client;

pub fn get_ping() -> Result<u32, String> {
    let address = "8.8.8.8"; // Google's DNS server
    let start = Instant::now();

    let output = if cfg!(target_os = "windows") {
        Command::new("ping")
            .args(&["-n", "1", "-w", "5000", address])
            .output()
    } else {
        Command::new("ping")
            .args(&["-c", "1", "-W", "5", address])
            .output()
    };

    match output {
        Ok(output) => {
            if output.status.success() {
                let duration = start.elapsed();
                Ok(duration.as_millis() as u32)
            } else {
                Err(format!("Ping failed: {}", String::from_utf8_lossy(&output.stderr)))
            }
        }
        Err(e) => Err(format!("Failed to execute ping: {}", e)),
    }
}

pub async fn get_public_ip() -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client.get("https://api.ipify.org").send().await?;

    if response.status().is_success() {
        Ok(response.text().await?)
    } else {
        Err(format!("Failed to get IP: HTTP {}", response.status()).into())
    }
}