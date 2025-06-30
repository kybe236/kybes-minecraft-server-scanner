mod blacklist;
mod config;
mod db;
mod packets;
mod utils;
mod worker;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use deadpool_postgres::{Manager, Pool};
use rand::random;
use tokio_postgres::NoTls;

use crate::{
    blacklist::{Blacklist, load_blacklist},
    config::Config,
    db::init::db_init,
    worker::handle_ip::handle_ip,
};

async fn start_scanning_workers(
    thread_count: usize,
    pool: Pool,
    blacklist: Arc<Blacklist>,
    config: Arc<Config>,
    rounds: u8,
    seed: u64,
    timeout_duration: Duration,
) {
    if thread_count == 0 {
        tracing::info!("worker_count is 0, scanning is disabled.");
        return;
    }

    let total_ips = u64::from(u32::MAX) + 1;
    let (tx, rx) = tokio::sync::mpsc::channel::<u32>(thread_count * 100);

    let mut handles = Vec::with_capacity(thread_count);
    let rx = Arc::new(tokio::sync::Mutex::new(rx));

    for _ in 0..thread_count {
        let rx = Arc::clone(&rx);
        let pool = pool.clone();
        let config = Arc::clone(&config);
        let blacklist = Arc::clone(&blacklist);
        let handle = tokio::spawn(async move {
            loop {
                let ip = {
                    let mut rx = rx.lock().await;
                    rx.recv().await
                };

                if let Some(ip) = ip {
                    let socket = SocketAddr::new(Ipv4Addr::from(ip).into(), 25565);
                    let _ = tokio::time::timeout(
                        timeout_duration,
                        handle_ip(
                            socket,
                            pool.clone(),
                            timeout_duration,
                            blacklist.clone(),
                            config.clone(),
                        ),
                    )
                    .await;
                } else {
                    break;
                }
            }
        });
        handles.push(handle);
    }

    let blacklist = blacklist.clone();
    tokio::spawn(async move {
        for i in 0..total_ips {
            let ip = permute_u32(i as u32, rounds, seed);
            let ip_addr = Ipv4Addr::from(ip);
            if !blacklist.contains(&ip_addr) && tx.send(ip).await.is_err() {
                break;
            }
        }
    })
    .await
    .unwrap();
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

    let pool = Pool::builder(mgr).max_size(100).build().unwrap();

    let client = pool.get().await.expect("Failed to get DB client");
    db_init(&client).await.expect("Failed to initialize DB");

    let seed: u64 = random();
    let rounds = 6;
    let thread_count = config.worker_count;
    let timeout_duration = Duration::from_millis(config.timeout_ms);
    let config = Arc::new(config);

    start_scanning_workers(
        thread_count,
        pool.clone(),
        Arc::clone(&blacklist),
        Arc::clone(&config),
        rounds,
        seed,
        timeout_duration,
    )
    .await;
}
