use std::{collections::HashSet, env};

use dotenv::dotenv;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    matrix::{matrix_join_room, matrix_log},
    string::write_string,
    u16::read_u16,
    varint::{read_var_int, read_var_int_from_stream, write_var_int},
};

mod matrix;
mod string;
mod u16;
mod varint;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let _ = matrix_join_room().await;

    let ignored_ips = env::var("IGNORED_IPS").unwrap_or_default();
    let ignored_ips: HashSet<_> = ignored_ips
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    let listener = TcpListener::bind("0.0.0.0:25565").await?;
    println!("Server listening on port 25565");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        let ip_str = addr.ip().to_string();

        if ignored_ips.contains(ip_str.as_str()) {
            println!("Ignoring connection from {}", ip_str);
            continue;
        }

        matrix_log(&format!(
            "New client connected: {}:{}",
            addr.ip(),
            addr.port()
        ))
        .await?;
        println!("New client: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = handle_client(&mut socket).await {
                eprintln!("Error handling client {}: {:?}", addr, e);
            }
        });
    }
}

async fn handle_client(stream: &mut TcpStream) -> anyhow::Result<()> {
    let mut status = false;
    loop {
        let packet_length = read_var_int_from_stream(stream).await?;
        if packet_length > 2_097_151 {
            anyhow::bail!("Packet length too large: {}", packet_length);
        }

        let mut buffer = vec![0u8; packet_length as usize];
        stream.read_exact(&mut buffer).await?;

        let mut offset = 0usize;
        let packet_id = {
            let id = read_var_int(&buffer.clone(), Some(&mut offset));
            id as i32
        };

        let data = &buffer[offset..];
        offset = 0;

        println!(
            "Received packet_id={} with {} bytes data",
            packet_id,
            data.len()
        );

        if !status && packet_id == 0x00 {
            let protocol_version = read_var_int(data, Some(&mut offset));
            let server_address = string::read_string(data, &mut offset)?;
            let server_port = read_u16(data, Some(&mut offset))?;
            let next_state = read_var_int(data, Some(&mut offset));

            println!(
                "Client connected with protocol version {}, address {}, port {}, next state {}",
                protocol_version, server_address, server_port, next_state
            );
            matrix_log(&format!(
                "Client connected with protocol version {}, address {}, port {}, next state {}",
                protocol_version, server_address, server_port, next_state
            ))
            .await?;

            status = true;
            continue;
        } else if status && packet_id == 0x00 {
            println!("sending payload");

            let data = include_str!("../msg.txt");

            let mut payload = Vec::new();
            write_var_int(&mut payload, &0x0);
            write_string(&mut payload, data);

            let mut response = Vec::new();
            write_var_int(&mut response, &(payload.len() as i32));
            response.extend_from_slice(&payload);

            if let Err(e) = stream.write_all(&response).await {
                let peer = stream
                    .peer_addr()
                    .unwrap_or_else(|_| "unknown".parse().unwrap());
                eprintln!("Failed to send response to client {}: {:?}", peer, e);
                return Err(e.into());
            }
        }
    }
}
