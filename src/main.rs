mod api;
mod app;
mod model;
mod theme;
mod ui;

use std::{io, path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{api::HereNowClient, app::App};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Override the here.now API origin.
    #[arg(long, default_value = "https://here.now")]
    base_url: String,

    /// Override the credentials file.
    #[arg(long)]
    credentials: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = HereNowClient::from_credentials(cli.base_url, cli.credentials)
        .context("could not initialize here.now authentication")?;
    let mut app = App::new(client);
    app.refresh().await;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app.run(&mut terminal, Duration::from_millis(180)).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}
