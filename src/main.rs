use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use tokio::task;
use tracing::info;

mod config;
mod entities;
mod llm;
mod p2p;
mod room_manager;
mod translation;
mod translation_service;
mod tui;

use crate::translation_service::disable_translation_worker;
use config::Config;
use tui::TuiApp;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to log file. If not provided, no logs will be emitted.
    #[arg(long)]
    log_file: Option<String>,

    /// Path to config file. If not provided, uses ~/.config/puf/config.toml
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    maybe_init_logging(&args)?;

    // Load configuration
    let config = Config::load(args.config.clone())?;
    info!(
        "Loaded config: disable_ai={}, username={}",
        config.disable_ai, config.username
    );

    // Conditionally load AI models based on config
    if !config.disable_ai {
        // Ensure all AI models are downloaded before taking over the terminal
        llm::ensure_ai_models_present().await?;
        // Warm LLMs in the background.
        // Future idea: Consider showing the state of this in the UI
        task::spawn(llm::warm_ai_models());
    } else {
        info!("AI/LLM functionality disabled by config");
        disable_translation_worker()?;
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(config);
    let res = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    info!("Application exited");
    Ok(())
}

fn maybe_init_logging(args: &Args) -> Result<()> {
    // Only initialize tracing if log-file is provided
    if let Some(log_file_path) = &args.log_file {
        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(&log_file_path).parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|err| {
                panic!("Failed to create directories for log file: {log_file_path}: {err}",)
            });
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file_path)
            .unwrap_or_else(|err| panic!("Failed to open log file: {log_file_path}: {err}"));

        tracing_subscriber::fmt()
            .with_writer(file)
            .with_ansi(false)
            .init();
    }

    Ok(())
}
