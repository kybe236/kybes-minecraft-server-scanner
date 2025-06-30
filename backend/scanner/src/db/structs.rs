use postgres_types::{FromSql, ToSql};
use serde_json::Value;

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Default, ToSql, FromSql,
)]
#[postgres(name = "players")]
pub struct Players {
    pub max: Option<i32>,
    pub online: Option<i32>,
    pub sample: Option<Vec<Player>>,
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Default, ToSql, FromSql,
)]
#[postgres(name = "player")]
pub struct Player {
    pub name: Option<String>,
    pub id: Option<String>,
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Default, ToSql, FromSql,
)]
#[postgres(name = "version")]
pub struct Version {
    pub name: Option<String>,
    pub protocol: Option<i32>,
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq, Hash, ToSql, FromSql,
)]
#[postgres(name = "action_type")]
pub enum ActionType {
    #[postgres(name = "JOINED")]
    Joined,
    #[postgres(name = "LEFT")]
    Left,
}

pub fn parse_players(value: &Value) -> Players {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

pub fn parse_version(value: &Value) -> Version {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

pub fn parse_description(value: &Value) -> String {
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

pub fn extract_players(players: Option<Players>) -> Vec<(String, String)> {
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

pub async fn get_user_id(client: &tokio_postgres::Client, name: &str, uuid: &str) -> Option<i32> {
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
