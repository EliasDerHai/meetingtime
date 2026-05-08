use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Event {
    pub title: String,
    pub start: DateTime<Utc>,
    pub join_url: Option<String>,
}

// Google Calendar API response shapes

#[derive(Deserialize)]
struct EventList {
    items: Option<Vec<RawEvent>>,
}

#[derive(Deserialize)]
struct RawEvent {
    summary: Option<String>,
    start: RawTime,
    #[serde(rename = "hangoutLink")]
    hangout_link: Option<String>,
    location: Option<String>,
    #[serde(default)]
    attendees: Vec<Attendee>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Attendee {
    #[serde(rename = "self", default)]
    is_self: bool,
    response_status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTime {
    date_time: Option<String>,
    date: Option<String>,
}

pub async fn fetch_upcoming(
    client: &reqwest::Client,
    access_token: &str,
) -> anyhow::Result<Vec<Event>> {
    let now = Utc::now();
    let time_max = now + Duration::hours(24);

    let resp = client
        .get("https://www.googleapis.com/calendar/v3/calendars/primary/events")
        .bearer_auth(access_token)
        .query(&[
            ("timeMin", now.to_rfc3339()),
            ("timeMax", time_max.to_rfc3339()),
            ("singleEvents", "true".to_string()),
            ("orderBy", "startTime".to_string()),
        ])
        .send()
        .await
        .context("failed to reach Google Calendar API")?
        .error_for_status()
        .context("Google Calendar API returned an error")?
        .json::<EventList>()
        .await
        .context("failed to parse Calendar API response")?;

    let events = resp
        .items
        .unwrap_or_default()
        .into_iter()
        .filter(|raw| {
            raw.attendees.is_empty()
                || raw
                    .attendees
                    .iter()
                    .any(|a| a.is_self && a.response_status == "accepted")
        })
        .filter_map(|raw| {
            let start_str = raw.start.date_time.or(raw.start.date)?;
            let start = DateTime::parse_from_rfc3339(&start_str)
                .ok()?
                .with_timezone(&Utc);
            let join_url = raw
                .hangout_link
                .or_else(|| raw.location.filter(|l| l.starts_with("http")));
            Some(Event {
                title: raw.summary.unwrap_or_else(|| "(no title)".to_string()),
                start,
                join_url,
            })
        })
        .collect();

    Ok(events)
}
