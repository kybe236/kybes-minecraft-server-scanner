use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddr},
    ops::RangeInclusive,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use ipnet::Ipv4Net;
use postgres_types::{FromSql, ToSql};
use rand::random;
use serde::Deserialize;
use serde_json::{self, Value};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
    time::timeout,
};
use tokio_postgres::{Client, NoTls};
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
    worker_recheck: u64,
    db_url: String,
    enable_isp_scan: bool,
    isp_scan_subnet: u8,
}

#[derive(Clone, Default)]
struct Blacklist {
    singles: HashSet<Ipv4Addr>,
    cidrs: Vec<Ipv4Net>,
    ranges: Vec<RangeInclusive<Ipv4Addr>>,
}

impl Blacklist {
    fn contains(&self, ip: &Ipv4Addr) -> bool {
        if self.singles.contains(ip) {
            return true;
        }
        for cidr in &self.cidrs {
            if cidr.contains(ip) {
                return true;
            }
        }
        for range in &self.ranges {
            if range.contains(ip) {
                return true;
            }
        }
        false
    }
}

fn parse_ip_range(range_str: &str) -> Option<RangeInclusive<Ipv4Addr>> {
    let parts: Vec<&str> = range_str.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let start = parts[0].parse::<Ipv4Addr>().ok()?;
    let end = parts[1].parse::<Ipv4Addr>().ok()?;
    if start <= end {
        Some(RangeInclusive::new(start, end))
    } else {
        Some(RangeInclusive::new(end, start))
    }
}

async fn load_blacklist(path: &str) -> Result<Blacklist, std::io::Error> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut singles = HashSet::new();
    let mut cidrs = Vec::new();
    let mut ranges = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Ok(ip) = line.parse::<Ipv4Addr>() {
            singles.insert(ip);
            continue;
        }
        if let Ok(cidr) = line.parse::<Ipv4Net>() {
            cidrs.push(cidr);
            continue;
        }
        if let Some(range) = parse_ip_range(line) {
            ranges.push(range);
            continue;
        }
        eprintln!("[WARN] Ignoring invalid blacklist line: {}", line);
    }
    Ok(Blacklist {
        singles,
        cidrs,
        ranges,
    })
}

fn ip_in_blacklist(ip: &Ipv4Addr, blacklist: &Blacklist) -> bool {
    blacklist.contains(ip)
}

fn handle_ip(
    addr: SocketAddr,
    db: Arc<Mutex<Client>>,
    timeout_duration: Duration,
    config: Arc<Config>,
    do_isp_scan: bool,
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
            // send the handshake packet
            let handshake = create_handshake_packet(757, &ip.to_string(), port, 1).await;
            if let Err(e) = stream.write_all(&handshake).await {
                tracing::warn!("{}:{} handshake failed: {}", ip, port, e);
                return;
            }

            // send the status packet
            let status = create_status_request().await;
            if let Err(e) = stream.write_all(&status).await {
                tracing::warn!("{}:{} status request failed: {}", ip, port, e);
                return;
            }

            // read the packet length
            let len = match read_var_int_from_stream(&mut stream).await {
                Ok(l) => l,
                Err(_) => {
                    return;
                }
            };

            // read the rest of the packet
            let mut buffer = vec![0; len as usize];
            if let Err(e) = stream.read_exact(&mut buffer).await {
                tracing::warn!("{}:{} read failed: {}", ip, port, e);
                return;
            }

            let mut index = 0;

            // read the packet ID
            let _ = read_var_int(&buffer, Some(&mut index));

            // read the response json
            let response = read_string(&buffer, &mut index).ok();

            // get access to the db
            let client = db.lock().await;

            if let Some(resp) = response {
                tracing::debug!("Got response for {}:{}", ip, port);
                save_json(&addr.to_string(), &resp, &client).await;
                drop(client);

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
                        let socket = SocketAddr::new(subnet_ip.into(), port);
                        let db = db.clone();
                        let config = Arc::clone(&config);
                        let timeout_duration = timeout_duration;
                        join_set.spawn(handle_ip(socket, db, timeout_duration, config, false));
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

fn feistel_round(x: u32, key: u32) -> u32 {
    let l = x >> 16;
    let r = x & 0xFFFF;
    let f = r.wrapping_mul(0x5bd1e995).rotate_left(13) ^ key;
    let new_l = r;
    let new_r = l ^ (f & 0xFFFF);
    (new_l << 16) | new_r
}

fn permute_u32(mut x: u32, rounds: u8, seed: u64) -> u32 {
    for i in 0..rounds {
        let key = (seed.wrapping_add(i as u64) & 0xFFFF_FFFF) as u32;
        x = feistel_round(x, key);
    }
    x
}

// Create the mc handshake packet
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

// Create the mc status request packet
async fn create_status_request() -> Vec<u8> {
    use crate::varint::write_var_int;
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    write_var_int(&mut inner, &0x0);
    write_var_int(&mut outer, &(inner.len() as i32));
    outer.extend_from_slice(&inner);
    outer
}

// This is going to save the json response into the db
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
                tracing::info!("Server rescanned: {} (id={})", addr, server_id);
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
                // Save joins
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
                // Save leaves
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
    value.as_str().unwrap_or_default().to_string()
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

// Converts a username to a offline uuid
// used to check if a account is likely cracked
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

async fn start_scanning_workers(
    thread_count: usize,
    db: Arc<Mutex<Client>>,
    blacklist: Blacklist,
    config: Arc<Config>,
    rounds: u8,
    seed: u64,
    timeout_duration: Duration,
) {
    if thread_count == 0 {
        tracing::info!("worker_count is 0, scanning is disabled.");
    } else {
        let db = Arc::clone(&db);
        let blacklist = blacklist.clone();
        let config = Arc::clone(&config);
        tokio::spawn(async move {
            let total_ips = u64::from(u32::MAX) + 1;
            let chunk_size = total_ips / thread_count as u64;
            let mut handles = Vec::with_capacity(thread_count);
            for t in 0..thread_count {
                let db = Arc::clone(&db);
                let blacklist = blacklist.clone();
                let config_worker = Arc::clone(&config);
                let start = t as u64 * chunk_size;
                let end = if t == thread_count - 1 {
                    total_ips
                } else {
                    (t as u64 + 1) * chunk_size
                };
                handles.push(tokio::spawn(async move {
                    let config = config_worker;
                    loop {
                        for i in start..end {
                            let ip = permute_u32(i as u32, rounds, seed);
                            let ip_addr = Ipv4Addr::from(ip);
                            if ip_in_blacklist(&ip_addr, &blacklist) {
                                continue;
                            }
                            let socket = SocketAddr::new(ip_addr.into(), 25565);
                            let config = Arc::clone(&config);
                            let _ = timeout(
                                timeout_duration,
                                handle_ip(socket, Arc::clone(&db), timeout_duration, config, true),
                            )
                            .await;
                        }
                        tracing::info!(
                            "[main scan] Finished scan cycle for chunk: {}-{}",
                            start,
                            end
                        );
                    }
                }));
            }
            for handle in handles {
                let res = handle.await;
                if res.is_err() {
                    tracing::error!("Task failed: {:?}", res);
                }
            }
        });
    }
}

async fn start_rescanner(db: Arc<Mutex<Client>>, blacklist: Blacklist, config: Arc<Config>) {
    if config.worker_recheck != 0 {
        let db = Arc::clone(&db);
        let blacklist = blacklist.clone();
        let config = Arc::clone(&config);
        tokio::spawn(async move {
            loop {
                let ips_ports: Vec<(Ipv4Addr, u16)> = {
                    let db_guard = db.lock().await;
                    let rows = db_guard.query("SELECT ip FROM servers", &[]).await;
                    match rows {
                        Ok(rows) => {
                            let mut ips_ports = Vec::new();
                            for row in rows.iter() {
                                let ip_port = row.get::<_, String>(0);
                                let mut split = ip_port.split(':');
                                let ip_part = split.next();
                                let port_part = split.next();
                                let ip: Option<Ipv4Addr> = ip_part.and_then(|s| s.parse().ok());
                                let port: u16 =
                                    port_part.and_then(|s| s.parse().ok()).unwrap_or(25565);
                                if let Some(ip) = ip {
                                    ips_ports.push((ip, port));
                                } else {
                                    tracing::warn!(
                                        "[rescanner] Could not parse IP from '{}', skipping",
                                        ip_port
                                    );
                                }
                            }
                            tracing::info!("[rescanner] Got {} IPs from DB", ips_ports.len());
                            ips_ports
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch IPs from DB: {}", e);
                            Vec::new()
                        }
                    }
                };
                let chunk_size = ips_ports.len().div_ceil(config.worker_recheck as usize);
                let mut handles = Vec::new();
                for chunk in ips_ports.chunks(chunk_size) {
                    let db = Arc::clone(&db);
                    let blacklist = blacklist.clone();
                    let config = Arc::clone(&config);
                    let chunk = chunk.to_vec();
                    let timeout_duration = Duration::from_millis(config.timeout_ms);
                    handles.push(tokio::spawn(async move {
                        for (ip, port) in &chunk {
                            if ip_in_blacklist(ip, &blacklist) {
                                continue;
                            }
                            let socket = SocketAddr::new((*ip).into(), *port);
                            handle_ip(
                                socket,
                                Arc::clone(&db),
                                timeout_duration,
                                Arc::clone(&config),
                                false,
                            )
                            .await;
                        }
                        tracing::info!(
                            "[rescanner] Finished recheck chunk of {} servers",
                            chunk.len()
                        );
                    }));
                }
                for handle in handles {
                    let _ = handle.await;
                }
            }
        });
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // start --test command
    let mut args = std::env::args().skip(1);
    if let Some(arg1) = args.next() {
        if arg1 == "--test" {
            if let Some(ip_str) = args.next() {
                let port = 25565u16;
                let ip: std::net::IpAddr = match ip_str.parse() {
                    Ok(ip) => ip,
                    Err(_) => {
                        eprintln!("Invalid IP address: {}", ip_str);
                        std::process::exit(1);
                    }
                };
                let socket = SocketAddr::new(ip, port);
                let config_data = tokio::fs::read_to_string("config.toml")
                    .await
                    .expect("Failed to read config.toml");
                let config: Config = toml::from_str(&config_data).expect("Invalid config format");
                let (client, connection) = tokio_postgres::connect(&config.db_url, NoTls)
                    .await
                    .expect("Failed to connect to DB");
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        tracing::error!("DB connection error: {}", e);
                    }
                });
                db_init(&client).await.expect("Failed to initialize DB");
                let db = Arc::new(Mutex::new(client));
                let timeout_duration = Duration::from_millis(config.timeout_ms);
                handle_ip(socket, db, timeout_duration, Arc::new(config), true).await;
                tracing::info!("[main] handle_ip finished");
                tokio::time::sleep(Duration::from_secs(2)).await;
                return;
            } else {
                eprintln!("Usage: --test <ip>");
                std::process::exit(1);
            }
        }
    }
    // end --test command

    // load the config
    let config_data = tokio::fs::read_to_string("config.toml")
        .await
        .expect("Failed to read config.toml");
    let config: Config = toml::from_str(&config_data).expect("Invalid config format");
    let blacklist = load_blacklist(&config.blacklist_file)
        .await
        .expect("Failed to load blacklist");

    // connect to the db
    let (client, connection) = tokio_postgres::connect(&config.db_url, NoTls)
        .await
        .expect("Failed to connect to DB");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("DB connection error: {}", e);
        }
    });

    // create tables if not existing
    db_init(&client).await.expect("Failed to initialize DB");
    let db = Arc::new(Mutex::new(client));

    // random seed for the ipv4 shuffler
    let seed: u64 = random();
    let rounds = 6;

    let thread_count = config.worker_count;
    let timeout_duration = Duration::from_millis(config.timeout_ms);
    let config = Arc::new(config);
    start_scanning_workers(
        thread_count,
        Arc::clone(&db),
        blacklist.clone(),
        Arc::clone(&config),
        rounds,
        seed,
        timeout_duration,
    )
    .await;

    start_rescanner(Arc::clone(&db), blacklist.clone(), Arc::clone(&config)).await;

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl_c");
}
