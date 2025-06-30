use std::{
    net::{Ipv4Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use deadpool_postgres::Pool;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

use crate::{
    blacklist::Blacklist,
    config::Config,
    db::save_json,
    packets::{
        create_handshake_packet, create_status_request,
        string::read_string,
        varint::{read_var_int, read_var_int_from_stream},
    },
    worker::scanner::scan_subnet_and_ports,
};

pub async fn try_handshake_and_status(
    stream: &mut TcpStream,
    ip: &str,
    port: u16,
) -> Result<String, String> {
    let handshake = create_handshake_packet(757, ip, port, 1).await;
    stream
        .write_all(&handshake)
        .await
        .map_err(|e| format!("handshake failed: {}", e))?;
    let status = create_status_request().await;
    stream
        .write_all(&status)
        .await
        .map_err(|e| format!("status request failed: {}", e))?;
    let len = read_var_int_from_stream(stream)
        .await
        .map_err(|_| "read_var_int_from_stream failed".to_string())?;
    let mut buffer = vec![0; len as usize];
    stream
        .read_exact(&mut buffer)
        .await
        .map_err(|e| format!("read failed: {}", e))?;
    let mut index = 0;
    let _ = read_var_int(&buffer, Some(&mut index));
    let response =
        read_string(&buffer, &mut index).map_err(|_| "read_string failed".to_string())?;
    Ok(response)
}

#[allow(dead_code)]
async fn extended_port_scan(
    ip: Ipv4Addr,
    pool: Pool,
    timeout_duration: Duration,
    _config: Arc<Config>,
    blacklist: Arc<Blacklist>,
) {
    let mut found = false;
    for port in 25500..=25700 {
        if blacklist.contains(&ip) {
            break;
        }
        let socket = SocketAddr::new(ip.into(), port);
        if try_port(socket, pool.clone(), timeout_duration, _config.clone()).await {
            found = true;
            break;
        }
    }
    if found {
        for port in 1024..=65535 {
            if blacklist.contains(&ip) {
                break;
            }
            let socket = SocketAddr::new(ip.into(), port);
            let _ = try_port(socket, pool.clone(), timeout_duration, _config.clone()).await;
        }
    }
}

#[allow(dead_code)]
async fn try_port(
    socket: SocketAddr,
    pool: Pool,
    timeout_duration: Duration,
    _config: Arc<Config>,
) -> bool {
    let ip = match socket.ip() {
        std::net::IpAddr::V4(ip) => ip,
        _ => return false,
    };
    let port = socket.port();
    if let Ok(Ok(mut stream)) =
        tokio::time::timeout(timeout_duration, TcpStream::connect(socket)).await
    {
        let handshake = create_handshake_packet(757, &ip.to_string(), port, 1).await;
        if stream.write_all(&handshake).await.is_err() {
            return false;
        }
        let status = create_status_request().await;
        if stream.write_all(&status).await.is_err() {
            return false;
        }
        let len = match read_var_int_from_stream(&mut stream).await {
            Ok(l) => l,
            Err(_) => return false,
        };
        let mut buffer = vec![0; len as usize];
        if stream.read_exact(&mut buffer).await.is_err() {
            return false;
        }
        let mut index = 0;
        let _ = read_var_int(&buffer, Some(&mut index));
        let response = read_string(&buffer, &mut index).ok();
        let client = match pool.get().await {
            Ok(c) => c,
            Err(_) => return false,
        };
        if let Some(resp) = response {
            save_json(&socket.to_string(), &resp, &client).await;
            return true;
        }
    }
    false
}

pub fn handle_ip(
    addr: SocketAddr,
    pool: Pool,
    timeout_duration: Duration,
    blacklist: Arc<Blacklist>,
    config: Arc<Config>,
) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        let ip = match addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => return,
        };

        if blacklist.contains(&ip) {
            tracing::debug!("{}:{} is in blacklist, skipping", ip, addr.port());
            return;
        }

        let port = addr.port();
        let stream_result = timeout(timeout_duration, TcpStream::connect(addr)).await;
        if let Ok(Ok(mut stream)) = stream_result {
            match try_handshake_and_status(&mut stream, &ip.to_string(), port).await {
                Ok(resp) => {
                    tracing::info!("Got response for {}:{}", ip, port);
                    if let Ok(client) = pool.get().await {
                        save_json(&addr.to_string(), &resp, &client).await;
                    }

                    if config.enable_isp_scan {
                        scan_subnet_and_ports(
                            ip,
                            pool.clone(),
                            timeout_duration,
                            config.clone(),
                            blacklist.clone(),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    tracing::warn!("{}:{}: {}", ip, port, e);
                }
            }
        }
    })
}
