use crate::{
    blacklist::Blacklist, config::Config, db::save_json,
    worker::handle_ip::try_handshake_and_status,
};
use deadpool_postgres::Pool;
use std::net::SocketAddr;
use std::{net::Ipv4Addr, sync::Arc, time::Duration};
use tokio::net::TcpStream;

pub async fn scan_subnet_and_ports(
    ip: Ipv4Addr,
    pool: Pool,
    timeout_duration: Duration,
    config: Arc<Config>,
    blacklist: Arc<Blacklist>,
) {
    let subnet_prefix = config.isp_scan_subnet;
    let extended_port_scan = config.extended_port_scan;
    let subnet = format!("{}/{}", ip, subnet_prefix);
    let net: ipnet::Ipv4Net = match subnet.parse() {
        Ok(n) => n,
        Err(_) => return,
    };
    let mut handles = vec![];
    for host in net.hosts() {
        if host == ip || blacklist.contains(&host) {
            continue;
        }
        let pool = pool.clone();
        let h = tokio::spawn(async move {
            let socket = SocketAddr::new(host.into(), 25565);
            let host_str = host.to_string();
            if let Ok(Ok(mut stream)) =
                tokio::time::timeout(timeout_duration, TcpStream::connect(socket)).await
            {
                if let Ok(resp) =
                    try_handshake_and_status(&mut stream, host_str.as_str(), 25565).await
                {
                    if let Ok(client) = pool.get().await {
                        save_json(&socket.to_string(), &resp, &client).await;
                        tracing::info!("[ISP SCAN] Found server at {}:25565", host);
                    }
                }
                if extended_port_scan {
                    for port in 1024..=65535u16 {
                        let port_socket = SocketAddr::new(host.into(), port);
                        let port_str = host.to_string();
                        if let Ok(Ok(mut port_stream)) =
                            tokio::time::timeout(timeout_duration, TcpStream::connect(port_socket))
                                .await
                        {
                            if let Ok(resp) =
                                try_handshake_and_status(&mut port_stream, port_str.as_str(), port)
                                    .await
                            {
                                if let Ok(client) = pool.get().await {
                                    save_json(&port_socket.to_string(), &resp, &client).await;
                                    tracing::info!(
                                        "[EXT PORT SCAN] Found server at {}:{}",
                                        host,
                                        port
                                    );
                                }
                            }
                        }
                    }
                }
            }
        });
        handles.push(h);
    }
    for h in handles {
        let _ = h.await;
    }
}
