use reqwest::Client;
use std::error::Error;
use std::net::IpAddr;
use surge_ping::{Client as PingClient, Config, PingIdentifier, PingSequence};
use tokio::time::{Duration, Instant};

pub async fn get_ping() -> Result<u32, String> {
    let address: IpAddr = "8.8.8.8"
        .parse()
        .map_err(|e| format!("Invalid IP address: {}", e))?;

    let config = Config::default();
    let client =
        PingClient::new(&config).map_err(|e| format!("Failed to create ping client: {}", e))?;

    let payload = [0; 56]; // Standard ping payload size
    let ident = PingIdentifier(rand::random());

    let mut pinger = client.pinger(address, ident).await;

    match pinger.ping(PingSequence(0), &payload).await {
        Ok((_, duration)) => Ok(duration.as_millis() as u32),
        Err(e) => Err(format!("Ping failed: {}", e)),
    }
}

pub async fn get_public_ip() -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

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
    use tokio::time::timeout;

    let client = Client::builder()
        .timeout(Duration::from_secs(10)) // Overall client timeout
        .build()?;

    let download_url = "https://speed.cloudflare.com/__down?bytes=100000000"; // 100MB file
    let upload_url = "https://speed.cloudflare.com/__up";
    let max_test_duration = Duration::from_secs(3);

    // Measure download speed with timeout
    let download_speed_mbps = {
        use std::sync::{
            atomic::{AtomicU64, Ordering},
            Arc,
        };

        let total_bytes = Arc::new(AtomicU64::new(0));
        let total_bytes_clone = total_bytes.clone();
        let start = Instant::now();

        match timeout(max_test_duration, async move {
            let mut response = client.get(download_url).send().await?;

            // Stream bytes and count them
            while let Some(chunk) = response.chunk().await? {
                total_bytes_clone.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            }

            Ok::<(), Box<dyn Error>>(())
        })
        .await
        {
            Ok(Ok(_)) | Ok(Err(_)) | Err(_) => {
                // Calculate speed regardless of completion or timeout
                let duration = start.elapsed();
                let bytes = total_bytes.load(Ordering::Relaxed);
                if bytes > 0 && duration.as_secs_f64() > 0.01 {
                    (bytes as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0)
                } else {
                    0.0
                }
            }
        }
    };

    // Measure upload speed with timeout
    let upload_speed_mbps = {
        use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
        use futures_util::stream::{self, StreamExt};

        let chunk_size = 65536; // 64KB chunks
        let total_bytes_to_send = Arc::new(AtomicU64::new(0));
        let total_bytes_to_send_clone = total_bytes_to_send.clone();
        let start = Instant::now();

        let upload_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        match timeout(max_test_duration, async move {
            // Create a stream that generates chunks and counts bytes
            let stream = stream::repeat_with(move || {
                let chunk = vec![0u8; chunk_size];
                total_bytes_to_send_clone.fetch_add(chunk_size as u64, Ordering::Relaxed);
                Ok::<_, std::io::Error>(chunk)
            })
            .take(500); // Max 500 chunks (32MB total)

            let body = reqwest::Body::wrap_stream(stream);

            upload_client.post(upload_url)
                .body(body)
                .send()
                .await?;

            Ok::<(), Box<dyn Error>>(())
        })
        .await
        {
            Ok(Ok(_)) | Err(_) => {
                // Calculate speed based on bytes actually sent
                let duration = start.elapsed();
                let bytes_sent = total_bytes_to_send.load(Ordering::Relaxed);
                if bytes_sent > 0 && duration.as_secs_f64() > 0.01 {
                    (bytes_sent as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0)
                } else {
                    0.0
                }
            }
            Ok(Err(_)) => {
                // Error during upload - return 0
                0.0
            }
        }
    };

    Ok((download_speed_mbps, upload_speed_mbps))
}
