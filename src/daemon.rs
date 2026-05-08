use anyhow::{Context, Result, anyhow};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{create_dir_all, read_to_string, write};
use tokio::time::sleep;

use crate::{
    auth,
    calendar::{self, Event},
    notify,
};

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    notify_before_meeting: Duration,
    auto_open_before_meeting: Duration,
    daemon_loop: Duration,
    daemon_idle_loop: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notify_before_meeting: Duration::minutes(5),
            auto_open_before_meeting: Duration::minutes(1),
            daemon_loop: Duration::seconds(15),
            daemon_idle_loop: Duration::minutes(5),
        }
    }
}

fn load_config() -> Result<Config> {
    let config_path = dirs::config_dir()
        .map(|base| base.join("meetingtime").join("config.json"))
        .ok_or_else(|| anyhow!("could not determine data directory"))?;

    let config = match config_path.try_exists()? {
        // exists -> load and return
        true => {
            let json = read_to_string(&config_path).with_context(|| {
                format!(
                    "config not found at {} — run `meetingtime auth` first",
                    config_path.display()
                )
            })?;

            serde_json::from_str(&json).context("config file is malformed")?
        }
        // doesn't exist -> save default and return default
        false => {
            if let Some(parent) = config_path.parent() {
                create_dir_all(parent)
                    .with_context(|| format!("could not create directory {}", parent.display()))?;
            }
            let default_config = Config::default();
            let json = serde_json::to_string_pretty(&default_config)?;
            write(&config_path, json).with_context(|| {
                format!(
                    "could not write default config to {}",
                    config_path.display()
                )
            })?;

            default_config
        }
    };

    Ok(config)
}

pub async fn run() -> anyhow::Result<()> {
    let mut notified: HashSet<Event> = HashSet::new();
    let mut opened: HashSet<Event> = HashSet::new();
    let mut token = auth::load_token()?;
    let config = load_config()?;
    let http_client = reqwest::Client::new();
    let (clicked_tx, mut clicked_rx) = tokio::sync::mpsc::channel::<Event>(8);

    loop {
        while let Ok(clicked_event) = clicked_rx.try_recv() {
            opened.insert(clicked_event);
        }

        token = auth::refresh_if_needed(token).await?;

        let events = match calendar::fetch_upcoming(&http_client, &token.access_token).await {
            Ok(e) => e,
            Err(e) => {
                eprintln!("fetch failed (will retry): {e:#}");
                sleep(config.daemon_loop.to_std()?).await;
                continue;
            }
        };
        let now = Utc::now();

        // Prune events that have already passed
        notified.retain(|e| e.start > now);
        opened.retain(|e| e.start > now);

        let next = events.into_iter().find(|e| e.start > now);

        let Some(event) = next else {
            println!(
                "No upcoming events in the next 24h - sleeping for {:?}.",
                config.daemon_idle_loop
            );
            sleep(config.daemon_idle_loop.to_std()?).await;
            continue;
        };

        if !notified.contains(&event) && event.start <= now + config.notify_before_meeting {
            notified.insert(event.clone());
            notify::fire(&event, clicked_tx.clone())?;
        }

        if !opened.contains(&event)
            && let Some(ref url) = event.join_url
            && event.start <= now + config.auto_open_before_meeting
        {
            opened.insert(event.clone());
            let _ = open::that(url);
        }

        sleep(config.daemon_loop.to_std()?).await;
    }
}
