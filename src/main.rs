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
use std::path::Path;
use tokio::task;
use tracing::info;

mod tui_app;
mod llm;
mod translation;

use tui_app::TuiApp;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to log file. If not provided, no logs will be emitted.
    #[arg(long)]
    log_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    maybe_init_logging(&args)?;

    // Warm LLMs in the background.
    // Future idea: Consider showing the state of this in the UI.
    task::spawn(llm::initialize_ai());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new();
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
            std::fs::create_dir_all(parent).expect(&format!(
                "Failed to create directories for log file: {}",
                log_file_path
            ));
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .expect(&format!("Failed to open log file: {}", log_file_path));

        tracing_subscriber::fmt()
            .with_writer(file)
            .with_ansi(false)
            .init();
    }

    Ok(())
}
