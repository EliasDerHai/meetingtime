use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "meetingtime",
    about = "Google Calendar meeting reminder daemon"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run OAuth2 PKCE auth flow and store token
    Auth,
    /// Long-running daemon loop (managed by systemd/launchd)
    Run,
    /// Print the next upcoming meeting and its join URL
    Next,
}
