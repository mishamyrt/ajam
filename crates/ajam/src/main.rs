mod logging;
mod state;
mod cli;

use ajam_profile::open_profiles;
use clap::Parser;
use fern::Dispatch;
use state::{State, StateConnect, StateEventsHandler};
use std::{process, path::Path};
use tokio::task;
use colored::Colorize;
use cli::Cli;

use ajam_events::ActivityMonitor;

fn setup_logging(verbose: bool, no_color: bool) {
    let log_level = if verbose { log::LevelFilter::Debug } else { log::LevelFilter::Info };
    Dispatch::new()
        .level(log_level)
        .chain(std::io::stdout())
        .apply()
        .expect("Unable to set up logger");

    if no_color {
        colored::control::set_override(false);
    }
}

#[tokio::main]
async fn main() -> process::ExitCode {
    let cli = Cli::parse();
    setup_logging(cli.verbose, cli.no_color);

    let profiles_dir = Path::new(&cli.profiles);
    let profiles = match open_profiles(profiles_dir) {
        Ok(profiles) => profiles,
        Err(e) => {
            print_error!("Failed to load profiles: {}", e);
            return process::ExitCode::FAILURE;
        }
    };
    print_info!("Loaded {} profiles", profiles.len());
    for (app_id, profile) in profiles.iter() {
        print_warning!("App: {} - {} pages", app_id, profile.pages.len());
    }

    let state = State::with_profiles(profiles);

    let (monitor, rx) = ActivityMonitor::new();

    let state_clone = state.clone();
    task::spawn(async move {
        print_debug!("Starting OS activity monitor listener");
        state_clone.listen_os_events(rx).await;
    });

    let state_clone = state.clone();
    task::spawn(async move {
        print_debug!("Starting device handler");
        let mut state_device = state_clone;
        state_device.connect_deck().await;
    });

    monitor.start_listening();

    process::ExitCode::SUCCESS
}
