mod string;
mod u16;
mod varint;

use deadpool_postgres::{ManagerConfig, Pool, RecyclingMethod, Runtime};
use postgres_types::{FromSql, ToSql};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::Ipv4Addr;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tokio_postgres::NoTls;
use tracing::error;

use crate::string::read_string;
use crate::varint::{read_var_int, read_var_int_from_stream};

#[derive(Debug, Deserialize)]
struct Config {
    ip_range: String,
    masscan_rate: u32,
    isp_scan_enabled: bool,
    mc_checker_threads: usize,
    database_url: String,
    timeout_ms: u64,
    blacklist_file: String,
    masscan_use_sudo: bool, // Run masscan with sudo if true
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
struct CompressedTarget {
    ip: u32,
    port: u16,
}

fn ipv4_to_u32(ip: Ipv4Addr) -> u32 {
    u32::from(ip)
}

fn u32_to_ipv4_string(ip: u32) -> String {
    Ipv4Addr::from(ip).to_string()
}

fn parse_masscan_output(stdout: impl BufRead) -> Vec<CompressedTarget> {
    let mut results = Vec::new();
    for line in stdout.lines() {
        if let Ok(l) = line {
            if l.starts_with("Discovered open port") {
                let parts: Vec<&str> = l.split_whitespace().collect();
                if parts.len() >= 6 {
                    if let Ok(port) = parts[3].split('/').next().unwrap_or("0").parse::<u16>() {
                        if let Ok(ip) = parts[5].parse::<Ipv4Addr>() {
                            results.push(CompressedTarget {
                                ip: ipv4_to_u32(ip),
                                port,
                            });
                        }
                    }
                }
            }
        }
    }
    results
}

fn run_masscan_custom(
    ip_range: &str,
    ports: &str,
    rate: u32,
    blacklist_file: &str,
    use_sudo: bool,
) -> Vec<CompressedTarget> {
    let mut cmd = if use_sudo {
        let mut c = Command::new("sudo");
        c.arg("masscan");
        c
    } else {
        Command::new("masscan")
    };
    cmd.arg(ip_range)
        .arg("-p")
        .arg(ports)
        .arg(format!("--rate={}", rate))
        .arg("--wait=0")
        .arg("--excludefile")
        .arg(blacklist_file)
        .stdout(Stdio::piped());

    let output = cmd
        .spawn()
        .expect("failed to run masscan")
        .stdout
        .expect("no stdout");
    let reader = BufReader::new(output);
    parse_masscan_output(reader)
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
        println!("[FOUND] Server: {}", addr);
        if let Some(players) = &players {
            println!("  Players: {:?}", players);
        }
        if let Some(version) = &version {
            println!("  Version: {:?}", version);
        }
        if let Some(favicon) = &favicon {
            println!(
                "  Favicon: {}...",
                &favicon.chars().take(30).collect::<String>()
            );
        }
        if let Some(enforces_secure_chat) = enforces_secure_chat {
            println!("  Enforces Secure Chat: {}", enforces_secure_chat);
        }
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
                Err(_) => {
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

#[derive(Debug, Clone, Deserialize, serde::Serialize, PartialEq, Default, ToSql, FromSql)]
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

async fn scan_server(ip: u32, port: u16, pool: &Arc<Pool>, timeout: u64) -> bool {
    let ip = u32_to_ipv4_string(ip);
    println!("🔍 Scanning {}:{}", ip, port);
    // Add detailed logging for each step
    match tokio::time::timeout(
        Duration::from_millis(timeout),
        TcpStream::connect((ip.clone(), port)),
    )
    .await
    {
        Ok(Ok(mut stream)) => {
            println!("🔗 Connected to {}:{}", ip, port);
            let handshake = create_handshake_packet(757, &ip.clone(), port, 1).await;
            if let Err(e) = stream.write_all(&handshake).await {
                println!("[ERROR] {}:{} handshake failed: {}", ip, port, e);
                return false;
            }

            println!("🤝 Handshake sent to {}:{}", ip, port);
            let status = create_status_request().await;
            if let Err(e) = stream.write_all(&status).await {
                println!("[ERROR] {}:{} status request failed: {}", ip, port, e);
                return false;
            }

            println!("📜 Status request sent to {}:{}", ip, port);
            let len = match read_var_int_from_stream(&mut stream).await {
                Ok(l) => l,
                Err(e) => {
                    println!("[ERROR] {}:{} failed to read varint: {}", ip, port, e);
                    return false;
                }
            };

            let mut buffer = vec![0; len as usize];
            if let Err(e) = stream.read_exact(&mut buffer).await {
                println!("[ERROR] {}:{} read failed: {}", ip, port, e);
                return false;
            }

            println!("📦 Received {} bytes from {}:{}", buffer.len(), ip, port);
            let mut index = 0;
            let _ = read_var_int(&buffer, Some(&mut index));
            let response = read_string(&buffer, &mut index).ok();
            let client = match pool.get().await {
                Ok(c) => c,
                Err(e) => {
                    println!("[ERROR] DB pool error: {}", e);
                    return false;
                }
            };

            if let Some(resp) = response {
                println!("✅ Got response for {}:{}", ip, port);
                save_json(&ip.to_string(), &resp, &client).await;
                return true;
            } else {
                println!(
                    "[ERROR] {}:{} response could not be parsed as string",
                    ip, port
                );
            }
        }
        Ok(Err(e)) => {
            println!("[ERROR] {}:{} TCP connect failed: {}", ip, port, e);
        }
        Err(e) => {
            println!("[ERROR] {}:{} TCP connect timeout: {}", ip, port, e);
        }
    }
    println!("❌ {}:{} is not a valid Minecraft server", ip, port);
    false
}

use std::future::Future;
use std::pin::Pin;

// Limit recursion depth to prevent infinite loops
const MAX_RECURSION_DEPTH: usize = 5;

#[allow(clippy::too_many_arguments)]
fn handle_ip_boxed(
    ip: u32,
    port: u16,
    config: Arc<Config>,
    advanced: bool,
    seen_ip_ports: Arc<Mutex<HashSet<(u32, u16)>>>,
    seen_ips: Arc<Mutex<HashMap<u32, u8>>>,
    pool: Arc<Pool>,
    depth: usize,
) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    // Only use pool in scan_server, never move it into recursion
    Box::pin(async move {
        if depth > MAX_RECURSION_DEPTH {
            println!(
                "[WARN] Max recursion depth reached for {}:{}",
                u32_to_ipv4_string(ip),
                port
            );
            return;
        }
        let ip_addr = Ipv4Addr::from(ip);
        let key = (ip, port);

        {
            let mut scanned = seen_ip_ports.lock().await;
            if scanned.contains(&key) {
                return;
            }
            scanned.insert(key);
        }

        println!("🔎 Pinging {}:{}", ip_addr, port);

        let is_mc_server = scan_server(ip, port, &pool, config.timeout_ms).await;
        if !is_mc_server {
            return;
        }

        let seen_count = {
            let mut ips = seen_ips.lock().await;
            let entry = ips.entry(ip).or_insert(0);
            *entry += 1;
            *entry
        };

        if seen_count == 1 {
            println!("⚠️ ROOT SCAN on {}", ip_addr);
            let ip_str = format!("{}", ip_addr);
            let results = run_masscan_custom(
                &ip_str,
                "1024-65535",
                config.masscan_rate,
                &config.blacklist_file,
                config.masscan_use_sudo,
            );
            for result in results {
                let pool_clone = pool.clone();
                handle_ip_boxed(
                    result.ip,
                    result.port,
                    config.clone(),
                    advanced,
                    seen_ip_ports.clone(),
                    seen_ips.clone(),
                    pool_clone,
                    depth + 1,
                )
                .await;
            }
            if advanced && config.isp_scan_enabled {
                println!("🌐 ISP Scan for {}", ip_addr);
                let subnet_range = format!(
                    "{}.{}.{}.0/24",
                    ip_addr.octets()[0],
                    ip_addr.octets()[1],
                    ip_addr.octets()[2]
                );
                let results = run_masscan_custom(
                    &subnet_range,
                    "2500-2600",
                    config.masscan_rate,
                    &config.blacklist_file,
                    config.masscan_use_sudo,
                );
                for result in results {
                    let ip_str = u32_to_ipv4_string(result.ip);
                    println!("⚠️ ROOT SCAN on ISP result {}", ip_str);
                    let root_results = run_masscan_custom(
                        &ip_str,
                        "1024-65535",
                        config.masscan_rate,
                        &config.blacklist_file,
                        config.masscan_use_sudo,
                    );
                    for result in root_results {
                        let pool_clone = pool.clone();
                        handle_ip_boxed(
                            result.ip,
                            result.port,
                            config.clone(),
                            advanced,
                            seen_ip_ports.clone(),
                            seen_ips.clone(),
                            pool_clone,
                            depth + 1,
                        )
                        .await;
                    }
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

#[tokio::main]
async fn main() {
    let raw = fs::read_to_string("config.toml").expect("Failed to read config.toml");
    let config: Config = toml::from_str(&raw).expect("Failed to parse TOML");
    let arc_config = Arc::new(config);

    let mut db_config = deadpool_postgres::Config::new();
    db_config.url = Some(arc_config.database_url.clone());
    db_config.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    let pool = db_config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("Failed to create pool");
    let pool = Arc::new(pool);

    // Initialize database
    let client = pool.get().await.expect("Failed to get database client");
    db_init(&client)
        .await
        .expect("Database initialization failed");

    let seen_ip_ports = Arc::new(Mutex::new(HashSet::new()));
    let seen_ips = Arc::new(Mutex::new(HashMap::new()));

    let targets = run_masscan_custom(
        &arc_config.ip_range,
        "25565",
        arc_config.masscan_rate,
        &arc_config.blacklist_file,
        arc_config.masscan_use_sudo,
    );

    let (tx, rx) = mpsc::channel::<CompressedTarget>(10000);

    // Only spawn one receiver loop, and share the receiver among all tasks
    let rx = Arc::new(Mutex::new(rx));
    for _ in 0..arc_config.mc_checker_threads {
        let rx = rx.clone();
        let cfg = arc_config.clone();
        let seen_ip_ports = seen_ip_ports.clone();
        let seen_ips = seen_ips.clone();
        let pool = pool.clone();

        tokio::spawn(async move {
            loop {
                let target = {
                    let mut rx = rx.lock().await;
                    rx.recv().await
                };
                if let Some(target) = target {
                    handle_ip_boxed(
                        target.ip,
                        target.port,
                        cfg.clone(),
                        true,
                        seen_ip_ports.clone(),
                        seen_ips.clone(),
                        pool.clone(),
                        0,
                    )
                    .await;
                } else {
                    break;
                }
            }
        });
    }

    for target in targets {
        println!(
            "🔎 Found target: {}:{}",
            u32_to_ipv4_string(target.ip),
            target.port
        );
        tx.send(target).await.unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    println!("✅ Done");
}
