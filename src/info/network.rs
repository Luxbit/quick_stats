use std::process::Command;
use std::error::Error;
use reqwest::Client;
use tokio::time::{Duration, Instant};
use rand::Rng;

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

pub async fn get_internet_speed() -> Result<(f64, f64), String> {
    match measure_internet_speed().await {
        Ok((download, upload)) => Ok((download, upload)),
        Err(e) => Err(format!("Failed to measure internet speed: {}", e)),
    }
}

pub async fn measure_internet_speed() -> Result<(f64, f64), Box<dyn Error>> {
    let client = Client::new();
    let download_url = "https://speed.cloudflare.com/__down?bytes=100000000"; // 100MB file
    let upload_url = "https://speed.cloudflare.com/__up";
    
    // Measure download speed
    let start = Instant::now();
    let response = client.get(download_url).send().await?;
    let bytes = response.bytes().await?;
    let download_duration = start.elapsed();
    let download_speed_mbps = (bytes.len() as f64 * 8.0) / (download_duration.as_secs_f64() * 1_000_000.0);

    // Measure upload speed
    let data_size = 10_000_000; // 10MB of data
    let data: Vec<u8> = (0..data_size).map(|_| rand::random::<u8>()).collect();
    let start = Instant::now();
    let _ = client.post(upload_url)
        .body(data)
        .send()
        .await?;
    let upload_duration = start.elapsed();
    let upload_speed_mbps = (data_size as f64 * 8.0) / (upload_duration.as_secs_f64() * 1_000_000.0);

    Ok((download_speed_mbps, upload_speed_mbps))
}
