use anyhow::Context;

pub fn fire(title: &str, join_url: Option<&str>) -> anyhow::Result<()> {
    let body = join_url.unwrap_or("No join link found");
    let summary = format!("Meeting starting soon: {title}");

    notify_rust::Notification::new()
        .summary(&summary)
        .body(body)
        .show()
        .context("failed to show notification")?;

    Ok(())
}
