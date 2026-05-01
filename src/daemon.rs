use chrono::{DateTime, Utc};
use tokio::time::sleep;

use crate::{auth, calendar, notify};

const REMINDER_MINUTES: i64 = 5;

pub async fn run() -> anyhow::Result<()> {
    let mut last_notified: Option<DateTime<Utc>> = None;

    loop {
        let token = auth::load_token()?;
        let token = auth::refresh_if_needed(token).await?;

        let events = calendar::fetch_upcoming(&token.access_token).await?;
        let now = Utc::now();

        let next = events
            .into_iter()
            .find(|e| e.start > now && last_notified.map_or(true, |t| e.start != t));

        let Some(event) = next else {
            println!("No upcoming events in the next 24h — rechecking in 30 minutes.");
            sleep(std::time::Duration::from_secs(30 * 60)).await;
            continue;
        };

        let notify_at = event.start - chrono::Duration::minutes(REMINDER_MINUTES);
        let wait = (notify_at - Utc::now()).to_std().unwrap_or_default();

        println!(
            "Next: \"{}\" at {} — notifying in {:.0}m",
            event.title,
            event.start.format("%H:%M UTC"),
            wait.as_secs_f64() / 60.0,
        );

        sleep(wait).await;

        notify::fire(&event.title, event.join_url.as_deref())?;
        last_notified = Some(event.start);

        // Sleep until 1 minute after the meeting starts before looping,
        // so we don't re-fire for the same event.
        let until_past = (event.start + chrono::Duration::minutes(1) - Utc::now())
            .to_std()
            .unwrap_or_default();
        sleep(until_past).await;
    }
}
