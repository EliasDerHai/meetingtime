use anyhow::Context;
use tokio::sync::mpsc::Sender;

use crate::calendar::Event;

pub fn fire(event: &Event, clicked_tx: Sender<Event>) -> anyhow::Result<()> {
    let summary = format!("Meeting starting soon: {}", event.title);

    let mut notification = notify_rust::Notification::new();
    notification.summary(&summary);

    let Some(ref url) = event.join_url else {
        notification
            .body("No join link found")
            .show()
            .context("failed to show notification")?;
        return Ok(());
    };

    notification.body(url).action("join", "Join");
    let handle = notification.show().context("failed to show notification")?;

    let url = url.to_owned();
    let event = event.clone();
    tokio::spawn(async move {
        let mut join_clicked = false;
        handle
            .wait_for_action_async(|action| {
                join_clicked = matches!(action, notify_rust::ActionResponse::Custom("join"));
            })
            .await;
        if join_clicked {
            let _ = open::that(&url);
            let _ = clicked_tx.send(event).await;
        }
    });

    Ok(())
}
