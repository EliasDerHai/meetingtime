use crate::{auth, calendar};

pub async fn run() -> anyhow::Result<()> {
    let token = auth::load_token()?;
    let token = auth::refresh_if_needed(token).await?;
    let events = calendar::fetch_upcoming(&token.access_token).await?;
    let now = chrono::Utc::now();
    match events.into_iter().find(|e| e.start > now) {
        Some(e) => {
            println!("Next meeting: {}", e.title);
            println!("Starts at:   {}", e.start.format("%Y-%m-%d %H:%M UTC"));
            match e.join_url {
                Some(url) => println!("Join URL:    {url}"),
                None => println!("Join URL:    (none found)"),
            }
        }
        None => println!("No upcoming meetings in the next 24 hours."),
    }
    Ok(())
}
