use std::env;

use anyhow::Result;
use reqwest::Client;

pub async fn matrix_log(message: &str) -> Result<()> {
    let homeserver = env::var("MATRIX_HOMESERVER")?;
    let access_token = env::var("MATRIX_ACCESS_TOKEN")?;
    let room_id = env::var("MATRIX_ROOM_ID")?;

    let url = format!(
        "{}/_matrix/client/r0/rooms/{}/send/m.room.message/{}",
        homeserver,
        room_id,
        uuid::Uuid::new_v4()
    );

    let client = Client::new();

    let body = serde_json::json!({
        "msgtype": "m.text",
        "body": message,
    });

    let res = client
        .put(&url)
        .query(&[("access_token", &access_token)])
        .json(&body)
        .send()
        .await?;

    if res.status().is_success() {
        println!("Sent matrix message: {}", message);
        Ok(())
    } else {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        anyhow::bail!("Matrix send failed: {} - {}", status, text);
    }
}

pub async fn matrix_join_room() -> Result<()> {
    let url = format!(
        "{}/_matrix/client/r0/join/{}",
        env::var("MATRIX_HOMESERVER")?,
        env::var("MATRIX_ROOM_ID")?
    );

    let client = Client::new();

    let res = client
        .post(&url)
        .query(&[("access_token", env::var("MATRIX_ACCESS_TOKEN")?)])
        .send()
        .await?;

    if res.status().is_success() {
        println!("Joined room {}", env::var("MATRIX_ROOM_ID")?);
        Ok(())
    } else {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        anyhow::bail!("Join room failed: {} - {}", status, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use std::env;

    #[tokio::test]
    async fn test_matrix_log() {
        dotenv().ok();

        let homeserver = env::var("MATRIX_HOMESERVER").unwrap_or_default();
        let access_token = env::var("MATRIX_ACCESS_TOKEN").unwrap_or_default();
        let room_id = env::var("MATRIX_ROOM_ID").unwrap_or_default();

        if homeserver.is_empty() || access_token.is_empty() || room_id.is_empty() {
            eprintln!("Skipping test_matrix_log because env vars are not set");
            return;
        }

        let result = matrix_log("Test message from automated test").await;
        assert!(result.is_ok(), "matrix_log failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_matrix_join_room() {
        dotenv().ok();

        let homeserver = env::var("MATRIX_HOMESERVER").unwrap_or_default();
        let access_token = env::var("MATRIX_ACCESS_TOKEN").unwrap_or_default();
        let room_id = env::var("MATRIX_ROOM_ID").unwrap_or_default();

        if homeserver.is_empty() || access_token.is_empty() || room_id.is_empty() {
            eprintln!("Skipping test_matrix_join_room because env vars are not set");
            return;
        }

        let result = matrix_join_room().await;
        assert!(
            result.is_ok(),
            "matrix_join_room failed: {:?}",
            result.err()
        );
    }
}
