use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub blacklist_file: String,
    pub worker_count: usize,
    pub timeout_ms: u64,
    pub db_url: String,
    pub enable_isp_scan: bool,
    pub isp_scan_subnet: u8,
    pub extended_port_scan: bool,
}
