use anyhow::Context;
use tokio::sync::mpsc::Sender;

use crate::calendar::Event;

pub fn fire(event: &Event, clicked_tx: Sender<Event>) -> anyhow::Result<()> {
    let summary = format!("Meeting starting soon: {}", event.title);

    let Some(ref url) = event.join_url else {
        notify_rust::Notification::new()
            .summary(&summary)
            .body("No join link found")
            .show()
            .context("failed to show notification")?;
        return Ok(());
    };

    let url = url.to_owned();
    let event = event.clone();

    #[cfg(target_os = "macos")]
    {
        // notify-rust's macOS backend discards the NotificationResponse from
        // mac_notification_sys::send(), so we drive mac_notification_sys directly.
        // send() is blocking — run it off the async executor.
        tokio::task::spawn_blocking(move || {
            use mac_notification_sys::{
                MainButton, Notification as MacNotif, NotificationResponse,
            };
            let mut n = MacNotif::default();
            n.title(&summary)
                .message(url.as_str())
                .main_button(MainButton::SingleAction("Join"));
            if let Ok(NotificationResponse::ActionButton(_)) = n.send() {
                let _ = open::that(&url);
                let _ = clicked_tx.blocking_send(event);
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        let mut notification = notify_rust::Notification::new();
        notification
            .summary(&summary)
            .body(&url)
            .action("join", "Join");
        let handle = notification.show().context("failed to show notification")?;
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
    }

    Ok(())
}
