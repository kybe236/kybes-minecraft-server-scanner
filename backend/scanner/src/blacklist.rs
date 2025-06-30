use std::net::Ipv4Addr;

use ipnet::Ipv4Net;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

#[derive(Clone, Default)]
pub struct Blacklist {
    cidrs: Vec<Ipv4Net>,
}

impl Blacklist {
    pub fn contains(&self, ip: &Ipv4Addr) -> bool {
        self.cidrs.iter().any(|cidr| cidr.contains(ip))
    }
}

pub fn range_to_cidrs(start: Ipv4Addr, end: Ipv4Addr) -> Vec<Ipv4Net> {
    let mut cidrs = Vec::new();
    let mut current = u32::from(start);
    let end = u32::from(end);
    while current <= end {
        let max_size = (!current).wrapping_add(1).trailing_zeros();
        let remaining = (end - current + 1).trailing_zeros();
        let prefix = 32 - max_size.min(remaining);
        let net = Ipv4Net::new(Ipv4Addr::from(current), prefix as u8).unwrap();
        cidrs.push(net);
        let hosts = 1u32 << (32 - prefix);
        current = current.saturating_add(hosts);
    }
    cidrs
}

pub fn parse_ip_range(range_str: &str) -> Option<(Ipv4Addr, Ipv4Addr)> {
    let parts: Vec<&str> = range_str.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let start = parts[0].parse::<Ipv4Addr>().ok()?;
    let end = parts[1].parse::<Ipv4Addr>().ok()?;
    if start <= end {
        Some((start, end))
    } else {
        Some((end, start))
    }
}

pub async fn load_blacklist(path: &str) -> Result<Blacklist, std::io::Error> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut cidrs = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Ok(ip) = line.parse::<Ipv4Addr>() {
            cidrs.push(Ipv4Net::new(ip, 32).unwrap());
            continue;
        }
        if let Ok(cidr) = line.parse::<Ipv4Net>() {
            cidrs.push(cidr);
            continue;
        }
        if let Some((start, end)) = parse_ip_range(line) {
            cidrs.extend(range_to_cidrs(start, end));
            continue;
        }
        eprintln!("[WARN] Ignoring invalid blacklist line: {}", line);
    }
    Ok(Blacklist { cidrs })
}
