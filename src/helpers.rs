pub fn bytes_to_megabytes(bytes: u64) -> u64 {
    (bytes as f64 / 1_048_576.0).round() as u64 // 1 MB = 1,048,576 bytes
}
