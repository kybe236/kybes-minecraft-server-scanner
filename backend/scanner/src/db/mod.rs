use std::collections::HashSet;

use serde_json::Value;

use crate::{
    db::structs::{
        ActionType, Players, Version, extract_players, get_user_id, parse_description,
        parse_players, parse_version,
    },
    utils::name_to_uuid,
};

pub mod init;
pub mod structs;

/// Parse and clean the server JSON, returning all extracted fields.
fn parse_server_json(json_str: &str) -> Option<ParsedServerJson> {
    let json_str = json_str.replace("\\u0000", "").replace('\u{0000}', "");
    let json: Value = serde_json::from_str(&json_str).ok()?;
    let description = json.get("description").cloned();
    let parsed_description = description.as_ref().map(parse_description);
    let enforces_secure_chat = json.get("enforcesSecureChat").and_then(|v| v.as_bool());
    let favicon = json
        .get("favicon")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let players = json.get("players").map(parse_players);
    let version = json.get("version").map(parse_version);
    let mut extra = json.clone();
    if let Some(obj) = extra.as_object_mut() {
        obj.remove("description");
        obj.remove("enforcesSecureChat");
        obj.remove("favicon");
        obj.remove("players");
        obj.remove("version");
    }
    Some(ParsedServerJson {
        parsed_description,
        raw_description: description,
        enforces_secure_chat,
        favicon,
        players,
        version,
        extra,
    })
}

struct ParsedServerJson {
    parsed_description: Option<String>,
    raw_description: Option<Value>,
    enforces_secure_chat: Option<bool>,
    favicon: Option<String>,
    players: Option<Players>,
    version: Option<Version>,
    extra: Value,
}

async fn upsert_server_row(
    addr: &str,
    parsed: &ParsedServerJson,
    client: &tokio_postgres::Client,
) -> Result<(i32, Option<Players>, bool), tokio_postgres::Error> {
    let existing = client
        .query_opt(
            "SELECT id, players FROM servers WHERE ip = $1 ORDER BY id DESC LIMIT 1;",
            &[&addr],
        )
        .await?;
    let raw_description_json = parsed.raw_description.clone().unwrap_or(Value::Null);
    if let Some(row) = existing {
        let old_players: Option<Players> = row.get("players");
        let updated_row = client
            .query_one(
                "UPDATE servers SET description = $2, raw_description = $3, players = $4, version = $5, favicon = $6, enforces_secure_chat = $7, extra = $8, last_pinged = NOW() WHERE id = $1 RETURNING id;",
                &[&row.get::<_, i32>("id"), &parsed.parsed_description, &raw_description_json, &parsed.players, &parsed.version, &parsed.favicon, &parsed.enforces_secure_chat, &parsed.extra],
            )
            .await?;
        let server_id = updated_row.get::<_, i32>("id");
        Ok((server_id, old_players, false))
    } else {
        let inserted_row = client
            .query_one(
                "INSERT INTO servers (ip, description, raw_description, players, version, favicon, enforces_secure_chat, extra) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id;",
                &[&addr, &parsed.parsed_description, &raw_description_json, &parsed.players, &parsed.version, &parsed.favicon, &parsed.enforces_secure_chat, &parsed.extra],
            )
            .await?;
        let server_id = inserted_row.get::<_, i32>("id");
        Ok((server_id, None, true))
    }
}

/// Save server JSON: orchestrates parsing, upserting, and player join/leave logic.
pub async fn save_json(addr: &str, json_str: &str, client: &tokio_postgres::Client) {
    let parsed = match parse_server_json(json_str) {
        Some(v) => v,
        None => return,
    };
    let (server_id, old_players_opt, is_new_server) =
        match upsert_server_row(addr, &parsed, client).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Error updating/inserting database: {}", e);
                return;
            }
        };
    if is_new_server {
        tracing::info!("Active server found: {}", addr);
    }
    if let Some(old_players) = old_players_opt {
        if let Some(players) = &parsed.players {
            if old_players != *players {
                save_player_leaves(&old_players, players, server_id, client).await;
            }
        }
    }
    save_player_joins(&parsed.players, server_id, client).await;
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
                    tracing::error!("Error getting player ID: {}", e);
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
