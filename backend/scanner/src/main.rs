use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddr},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use deadpool_postgres::{Manager, Pool};
use ipnet::Ipv4Net;
use postgres_types::{FromSql, ToSql};
use serde::Deserialize;
use serde_json::{self, Value};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    process::Command,
};
use tokio_postgres::NoTls;
use tracing::{error, info};

mod string;
mod u16;
mod varint;
use string::read_string;
use varint::{read_var_int, read_var_int_from_stream};

#[derive(Deserialize)]
struct Config {
    blacklist_file: String,
    worker_count: usize,
    timeout_ms: u64,
    db_url: String,
    enable_isp_scan: bool,
    isp_scan_subnet: u8,
    extended_port_scan: bool,
    concurrency_per_worker: usize,
    scan_rate: u32,
}

#[derive(Clone, Default)]
struct Blacklist {
    cidrs: Vec<Ipv4Net>,
}

impl Blacklist {
    fn contains(&self, ip: &Ipv4Addr) -> bool {
        self.cidrs.iter().any(|cidr| cidr.contains(ip))
    }
}

fn range_to_cidrs(start: Ipv4Addr, end: Ipv4Addr) -> Vec<Ipv4Net> {
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

fn parse_ip_range(range_str: &str) -> Option<(Ipv4Addr, Ipv4Addr)> {
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

async fn load_blacklist(path: &str) -> Result<Blacklist, std::io::Error> {
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

fn handle_ip(
    addr: SocketAddr,
    pool: Pool,
    timeout_duration: Duration,
    config: Arc<Config>,
    do_isp_scan: bool,
    blacklist: Arc<Blacklist>,
) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        let ip = match addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => return,
        };
        let port = addr.port();
        if let Ok(Ok(mut stream)) =
            tokio::time::timeout(timeout_duration, TcpStream::connect(addr)).await
        {
            let handshake = create_handshake_packet(757, &ip.to_string(), port, 1).await;
            if let Err(e) = stream.write_all(&handshake).await {
                tracing::warn!("{}:{} handshake failed: {}", ip, port, e);
                return;
            }
            let status = create_status_request().await;
            if let Err(e) = stream.write_all(&status).await {
                tracing::warn!("{}:{} status request failed: {}", ip, port, e);
                return;
            }
            let len = match read_var_int_from_stream(&mut stream).await {
                Ok(l) => l,
                Err(_) => {
                    return;
                }
            };
            let mut buffer = vec![0; len as usize];
            if let Err(e) = stream.read_exact(&mut buffer).await {
                tracing::warn!("{}:{} read failed: {}", ip, port, e);
                return;
            }
            let mut index = 0;
            let _ = read_var_int(&buffer, Some(&mut index));
            let response = read_string(&buffer, &mut index).ok();
            let client = match pool.get().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("DB pool error: {}", e);
                    return;
                }
            };
            if let Some(resp) = response {
                tracing::debug!("Got response for {}:{}", ip, port);
                save_json(&addr.to_string(), &resp, &client).await;
                if config.enable_isp_scan && do_isp_scan {
                    info!("{}:{} ISP scan enabled, scanning subnet", ip, port);
                    let prefix = config.isp_scan_subnet;
                    let net = ipnet::Ipv4Net::new(ip, prefix)
                        .unwrap_or_else(|_| ipnet::Ipv4Net::new(ip, 24).unwrap());
                    use tokio::task::JoinSet;
                    let mut join_set = JoinSet::new();
                    let max_concurrent = 32;
                    for subnet_ip in net.hosts() {
                        if subnet_ip == ip {
                            continue;
                        }
                        let pool = pool.clone();
                        let config = Arc::clone(&config);
                        let timeout_duration = timeout_duration;
                        let blacklist = Arc::clone(&blacklist);
                        if config.extended_port_scan {
                            join_set.spawn(extended_port_scan(
                                subnet_ip,
                                pool,
                                timeout_duration,
                                config,
                                Arc::clone(&blacklist),
                            ));
                        } else {
                            let socket = SocketAddr::new(subnet_ip.into(), port);
                            join_set.spawn(handle_ip(
                                socket,
                                pool,
                                timeout_duration,
                                config,
                                false,
                                Arc::clone(&blacklist),
                            ));
                        }
                        if join_set.len() >= max_concurrent {
                            let _ = join_set.join_next().await;
                        }
                    }
                    while join_set.join_next().await.is_some() {}
                }
            }
        }
    })
}

pub async fn db_init(client: &tokio_postgres::Client) -> Result<(), tokio_postgres::Error> {
    client
        .batch_execute(
            r#"
        -- Conditionally create custom types if they do not exist
        DO $$ BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'action_type') THEN
                CREATE TYPE action_type AS ENUM ('JOINED', 'LEFT');
            END IF;
        END$$;
        DO $$ BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'player') THEN
                CREATE TYPE player AS (
                    name TEXT,
                    id TEXT
                );
            END IF;
        END$$;
        DO $$ BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'version') THEN
                CREATE TYPE version AS (
                    name TEXT,
                    protocol INTEGER
                );
            END IF;
        END$$;
        DO $$ BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'players') THEN
                CREATE TYPE players AS (
                    max INTEGER,
                    online INTEGER,
                    sample player[]
                );
            END IF;
        END$$;

        -- Create servers table
        CREATE TABLE IF NOT EXISTS servers (
            id SERIAL PRIMARY KEY,
            ip TEXT NOT NULL UNIQUE,
            description TEXT,
            raw_description JSONB,
            players players,
            version version,
            favicon TEXT,
            enforces_secure_chat BOOLEAN,
            extra JSONB,
            last_pinged TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        -- Create player list table
        CREATE TABLE IF NOT EXISTS player_list (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            uuid TEXT NOT NULL,
            cracked BOOLEAN NOT NULL,
            UNIQUE (uuid, name)
        );

        -- Create player actions table
        CREATE TABLE IF NOT EXISTS player_actions (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES player_list(id),
            server_id INTEGER NOT NULL REFERENCES servers(id),
            action action_type NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        -- Create validator status table (for validator only)
        CREATE TABLE IF NOT EXISTS validator_status (
            id SERIAL PRIMARY KEY,
            ips_validated INTEGER NOT NULL,
            ips_active INTEGER NOT NULL,
            ips_validated_list TEXT[] NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        -- Create scanner status table (for scanner only)
        CREATE TABLE IF NOT EXISTS status (
            id SERIAL PRIMARY KEY,
            ips_scanned INTEGER NOT NULL,
            ips_active INTEGER NOT NULL,
            ips_active_list TEXT[] NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        -- Create index on player_list for faster lookups
        CREATE INDEX IF NOT EXISTS idx_player_list_name_uuid ON player_list (name, uuid);
        "#,
        )
        .await?;

    Ok(())
}

async fn create_handshake_packet(
    protocol_version: i32,
    server_address: &str,
    server_port: u16,
    next_state: i32,
) -> Vec<u8> {
    use crate::string::write_string;
    use crate::u16::write_u16;
    use crate::varint::write_var_int;
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    write_var_int(&mut inner, &0x0);
    write_var_int(&mut inner, &protocol_version);
    write_string(&mut inner, server_address);
    write_u16(&mut inner, server_port);
    write_var_int(&mut inner, &next_state);
    write_var_int(&mut outer, &(inner.len() as i32));
    outer.extend_from_slice(&inner);
    outer
}

async fn create_status_request() -> Vec<u8> {
    use crate::varint::write_var_int;
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    write_var_int(&mut inner, &0x0);
    write_var_int(&mut outer, &(inner.len() as i32));
    outer.extend_from_slice(&inner);
    outer
}

async fn save_json(addr: &str, json_str: &str, client: &tokio_postgres::Client) {
    let json_str = json_str.replace("\\u0000", "").replace('\u{0000}', "");
    let json = serde_json::from_str(&json_str);
    let mut json: Value = match json {
        Ok(v) => v,
        Err(e) => {
            error!("save_json: JSON parse error: {}", e);
            return;
        }
    };
    let description = json.get("description").cloned();
    json.as_object_mut().map(|obj| obj.remove("description"));
    let parsed_description = parse_description(&description.clone().unwrap_or_default());
    let enforces_secure_chat = json.get("enforcesSecureChat").and_then(|v| v.as_bool());
    json.as_object_mut()
        .map(|obj| obj.remove("enforcesSecureChat"));
    let favicon = json
        .get("favicon")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    json.as_object_mut().map(|obj| obj.remove("favicon"));
    let players = json.get("players").map(parse_players);
    json.as_object_mut().map(|obj| obj.remove("players"));
    let version = json.get("version").map(parse_version);
    json.as_object_mut().map(|obj| obj.remove("version"));
    let extra = json;
    let extra_json = serde_json::to_value(extra);
    let extra_json = match extra_json {
        Ok(v) => v,
        Err(_) => {
            return;
        }
    };
    let existing_result = client
        .query_opt(
            "SELECT id, players FROM servers WHERE ip = $1 ORDER BY id DESC LIMIT 1;",
            &[&addr],
        )
        .await;
    let existing = match existing_result {
        Ok(row) => row,
        Err(e) => {
            error!("Error querying database: {}", e);
            return;
        }
    };

    let (server_id, old_players_opt);
    let mut is_new_server = false;
    if let Some(row) = existing {
        let old_players: Option<Players> = row.get::<_, Option<Players>>("players");
        let updated_row = client
            .query_one(
                r#"
                    UPDATE servers
                    SET description = $2,
                        raw_description = $3,
                        players = $4,
                        version = $5,
                        favicon = $6,
                        enforces_secure_chat = $7,
                        extra = $8,
                        last_pinged = NOW()
                    WHERE ip = $1
                    RETURNING id
                "#,
                &[
                    &addr,
                    &parsed_description,
                    &description,
                    &players,
                    &version,
                    &favicon,
                    &enforces_secure_chat,
                    &extra_json,
                ],
            )
            .await;
        match updated_row {
            Ok(row) => {
                server_id = row.get::<_, i32>("id");
                old_players_opt = old_players;
            }
            Err(e) => {
                error!("Error updating database: {}", e);
                return;
            }
        }
    } else {
        let inserted_row = client
            .query_one(
                r#"
                    INSERT INTO servers (
                        ip,
                        description,
                        raw_description,
                        players,
                        version,
                        favicon,
                        enforces_secure_chat,
                        extra,
                        last_pinged
                    ) VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, NOW()
                    )
                    RETURNING id
                "#,
                &[
                    &addr,
                    &parsed_description,
                    &description,
                    &players,
                    &version,
                    &favicon,
                    &enforces_secure_chat,
                    &extra_json,
                ],
            )
            .await;
        match inserted_row {
            Ok(row) => {
                server_id = row.get::<_, i32>("id");
                old_players_opt = None;
                is_new_server = true;
                tracing::info!("New server added: {} (id={})", addr, server_id);
            }
            Err(e) => {
                error!("Error inserting into database: {}", e);
                return;
            }
        }
        save_player_joins(&players, server_id, client).await;
    }

    if is_new_server {
        tracing::info!("Active server found: {}", addr);
    }

    if let Some(old_players) = old_players_opt {
        if let Some(players) = players {
            if old_players != players {
                let old_set: HashSet<_> = extract_players(Some(old_players.clone()))
                    .into_iter()
                    .collect();
                let new_set: HashSet<_> =
                    extract_players(Some(players.clone())).into_iter().collect();
                for (name, uuid) in new_set.difference(&old_set) {
                    if name.trim().is_empty() || uuid.trim().is_empty() {
                        continue;
                    }
                    if name.contains(" ") || uuid.contains(" ") {
                        continue;
                    }
                    if name.contains("§") || uuid.contains("§") {
                        continue;
                    }
                    if name.contains(".") || uuid.contains(".") {
                        continue;
                    }
                    let mut user_id = get_user_id(client, name, uuid).await;
                    if user_id.is_none() {
                        let row = client
                            .query_one(
                                r#"
                                    INSERT INTO player_list (name, uuid, cracked)
                                    VALUES ($1, $2, $3)
                                    ON CONFLICT (uuid, name) DO NOTHING
                                    RETURNING id
                                "#,
                                &[name, uuid, &(name_to_uuid(name) == *uuid)],
                            )
                            .await;
                        let row = match row {
                            Ok(row) => row,
                            Err(_) => {
                                continue;
                            }
                        };
                        let id = row.try_get("id");
                        let id = match id {
                            Ok(id) => id,
                            Err(e) => {
                                error!("Error getting player ID: {}", e);
                                continue;
                            }
                        };
                        user_id = Some(id);
                    }
                    if let Some(user_id) = user_id {
                        let _ = client
                            .execute(
                                r#"
                                    INSERT INTO player_actions (user_id, server_id, action)
                                    VALUES ($1, $2, $3)
                                "#,
                                &[&user_id, &server_id, &ActionType::Joined],
                            )
                            .await;
                    }
                }
                save_player_leaves(&old_players, &players, server_id, client).await;
            }
        }
    }
}

async fn save_player_joins(
    players: &Option<Players>,
    server_id: i32,
    client: &tokio_postgres::Client,
) {
    if let Some(players) = players {
        if let Some(sample) = &players.sample {
            for player in sample {
                let name = match player.name.clone() {
                    Some(name) => name,
                    None => continue,
                };
                let id = match player.id.clone() {
                    Some(id) => id,
                    None => continue,
                };
                if name.trim().is_empty() || id.trim().is_empty() {
                    continue;
                }
                if name.contains(" ")
                    || id.contains(" ")
                    || name.contains("§")
                    || id.contains("§")
                    || name.contains(".")
                    || id.contains(".")
                {
                    continue;
                }
                let user_id = match get_user_id(client, &name, &id).await {
                    Some(uid) => uid,
                    None => {
                        let row = client
                            .query_one(
                                r#"
                                    INSERT INTO player_list (name, uuid, cracked)
                                    VALUES ($1, $2, $3)
                                    ON CONFLICT (uuid, name) DO NOTHING
                                    RETURNING id
                                "#,
                                &[&name, &id, &(name_to_uuid(&name) == id)],
                            )
                            .await;
                        match row {
                            Ok(row) => row.get("id"),
                            Err(_) => continue,
                        }
                    }
                };
                let _ = client
                    .execute(
                        r#"
                            INSERT INTO player_actions (user_id, server_id, action)
                            VALUES ($1, $2, $3)
                        "#,
                        &[&user_id, &server_id, &ActionType::Joined],
                    )
                    .await;
            }
        }
    }
}

async fn save_player_leaves(
    old_players: &Players,
    players: &Players,
    server_id: i32,
    client: &tokio_postgres::Client,
) {
    let old_set: HashSet<_> = extract_players(Some(old_players.clone()))
        .into_iter()
        .collect();
    let new_set: HashSet<_> = extract_players(Some(players.clone())).into_iter().collect();
    for (name, id) in old_set.difference(&new_set) {
        if name.trim().is_empty() || id.trim().is_empty() {
            continue;
        }
        if name.contains(" ")
            || id.contains(" ")
            || name.contains("§")
            || id.contains("§")
            || name.contains(".")
            || id.contains(".")
        {
            continue;
        }
        let mut user_id = get_user_id(client, name, id).await;
        if user_id.is_none() {
            let row = client
                .query_one(
                    r#"
                        INSERT INTO player_list (name, uuid, cracked)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (uuid, name) DO NOTHING
                        RETURNING id
                    "#,
                    &[name, id, &(name_to_uuid(name) == *id)],
                )
                .await;
            let row = match row {
                Ok(row) => row,
                Err(_) => {
                    continue;
                }
            };
            let res = row.try_get("id");
            let res = match res {
                Ok(id) => id,
                Err(e) => {
                    error!("Error getting player ID: {}", e);
                    continue;
                }
            };
            user_id = Some(res);
        }
        if let Some(user_id) = user_id {
            let _ = client
                .execute(
                    "INSERT INTO player_actions (user_id, server_id, action) VALUES ($1, $2, $3)",
                    &[&user_id, &server_id, &ActionType::Left],
                )
                .await;
        }
    }
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Default, ToSql, FromSql,
)]
#[postgres(name = "players")]
struct Players {
    max: Option<i32>,
    online: Option<i32>,
    sample: Option<Vec<Player>>,
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Default, ToSql, FromSql,
)]
#[postgres(name = "player")]
struct Player {
    name: Option<String>,
    id: Option<String>,
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Default, ToSql, FromSql,
)]
#[postgres(name = "version")]
struct Version {
    name: Option<String>,
    protocol: Option<i32>,
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq, Hash, ToSql, FromSql,
)]
#[postgres(name = "action_type")]
enum ActionType {
    #[postgres(name = "JOINED")]
    Joined,
    #[postgres(name = "LEFT")]
    Left,
}

fn parse_players(value: &Value) -> Players {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

fn parse_version(value: &Value) -> Version {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

fn parse_description(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        return s.to_string();
    }

    fn extract_text(val: &Value) -> String {
        if let Some(s) = val.get("text").and_then(|v| v.as_str()) {
            let mut out = s.to_string();
            if let Some(extra) = val.get("extra").and_then(|v| v.as_array()) {
                for e in extra {
                    out.push_str(&extract_text(e));
                }
            }
            return out;
        }

        if let Some(arr) = val.as_array() {
            return arr.iter().map(extract_text).collect();
        }
        String::new()
    }
    extract_text(value)
}

fn extract_players(players: Option<Players>) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Some(players) = players {
        if let Some(sample) = players.sample {
            for player in sample {
                if let (Some(name), Some(id)) = (player.name, player.id) {
                    result.push((name, id));
                }
            }
        }
    }
    result
}

async fn get_user_id(client: &tokio_postgres::Client, name: &str, uuid: &str) -> Option<i32> {
    let row = client
        .query_one(
            "SELECT id FROM player_list WHERE name = $1 AND uuid = $2",
            &[&name, &uuid],
        )
        .await
        .ok()?;
    let id: i32 = row.get("id");
    Some(id)
}

pub fn name_to_uuid(username: &str) -> String {
    let mut hash = md5::compute(format!("OfflinePlayer:{}", username)).0;
    hash[6] = hash[6] & 0x0f | 0x30;
    hash[8] = hash[8] & 0x3f | 0x80;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]),
        u16::from_be_bytes([hash[4], hash[5]]),
        u16::from_be_bytes([hash[6], hash[7]]),
        u16::from_be_bytes([hash[8], hash[9]]),
        u64::from_be_bytes([
            hash[10], hash[11], hash[12], hash[13], hash[14], hash[15], 0, 0
        ]) >> 16
    )
}

async fn run_masscan(command: &str, output_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(format!("{} > {}", command, output_file));
    let status = cmd.status().await?;
    if !status.success() {
        return Err(format!("masscan failed with status: {}", status).into());
    }
    Ok(())
}

async fn process_masscan_json(
    json_file: &str,
    pool: Pool,
    blacklist: Arc<Blacklist>,
    config: Arc<Config>,
    timeout_duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(json_file).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(
        config.worker_count * config.concurrency_per_worker,
    ));
    while let Some(line) = lines.next_line().await? {
        if !line.trim().starts_with('{') {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&line)?;
        if let (Some(ip), Some(port)) = (value["ip"].as_str(), value["ports"].as_array()) {
            let ip: Ipv4Addr = ip.parse()?;
            for port_obj in port {
                if let Some(port_num) = port_obj["port"].as_u64() {
                    let port = port_num as u16;
                    if blacklist.contains(&ip) {
                        continue;
                    }
                    let socket = SocketAddr::new(ip.into(), port);
                    let permit = semaphore.clone().acquire_owned().await?;
                    let pool = pool.clone();
                    let config = config.clone();
                    let blacklist = blacklist.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        handle_ip(socket, pool, timeout_duration, config, true, blacklist).await;
                    });
                }
            }
        }
    }
    // Wait for all tasks to complete
    let permits = config.worker_count * config.concurrency_per_worker;
    for _ in 0..permits {
        let _ = semaphore.acquire().await;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let config_data = tokio::fs::read_to_string("config.toml")
        .await
        .expect("Failed to read config.toml");
    let config: Config = toml::from_str(&config_data).expect("Invalid config format");
    let blacklist = Arc::new(
        load_blacklist(&config.blacklist_file)
            .await
            .expect("Failed to load blacklist"),
    );
    let pg_config = config
        .db_url
        .parse::<tokio_postgres::Config>()
        .expect("Invalid db_url");
    let mgr = Manager::new(pg_config, NoTls);
    let pool = Pool::builder(mgr).build().unwrap();
    let client = pool.get().await.expect("Failed to get DB client");
    db_init(&client).await.expect("Failed to initialize DB");
    let timeout_duration = Duration::from_millis(config.timeout_ms);
    let config = Arc::new(config);
    // Run masscan and save JSON results
    let masscan_output = "masscan_output.json";
    let masscan_command = format!(
        "sudo masscan 0.0.0.0/0 -p25565 --rate {} -oJ - --excludefile {}",
        config.scan_rate, config.blacklist_file
    );
    if let Err(e) = run_masscan(&masscan_command, masscan_output).await {
        error!("Masscan failed: {}", e);
        return;
    }
    process_masscan_json(
        masscan_output,
        pool.clone(),
        Arc::clone(&blacklist),
        Arc::clone(&config),
        timeout_duration,
    )
    .await
    .expect("Failed to process masscan results");
    // Cleanup temporary file
    let _ = tokio::fs::remove_file(masscan_output).await;
}
