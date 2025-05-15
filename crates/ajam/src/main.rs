mod logging;
mod state;
mod cli;

use ajam_launchctl::{LaunchAgent, LaunchControllable};
use ajam_profile::open_profiles;
use clap::Parser;
use fern::Dispatch;
use state::{ActivityHandler, State, StateConnect};
use std::{path::{Path, PathBuf}, process};
use tokio::{task, signal};
use colored::Colorize;
use cli::{Cli, Command};
use ajam_activity::Monitor;

const APP_LABEL: &str = "co.myrt.ajam";

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

async fn run_listener(profiles_dir: &str) -> process::ExitCode {
    let profiles_dir = Path::new(&profiles_dir);
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

    let (monitor, rx) = Monitor::new();

    let state_clone = state.clone();
    task::spawn(async move {
        print_debug!("Starting OS activity monitor listener");
        state_clone.listen_activity_events(rx).await;
    });

    let state_clone = state.clone();
    task::spawn(async move {
        print_debug!("Starting device handler");
        let mut state_device = state_clone;
        state_device.connect_deck().await;
    });

    let state_clone = state.clone();
    task::spawn(async move {
        handle_signals(state_clone).await;
    });

    if let Err(e) = monitor.start_listening() {
        print_error!("Failed to start activity monitor: {}", e);
    }

    process::ExitCode::SUCCESS
}

async fn handle_signals(state: State) {
    let mut term_signal = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to create SIGTERM signal handler");
    
    tokio::select! {
        _ = signal::ctrl_c() => {
            print_info!("Received Ctrl+C, shutting down");
        }
        _ = term_signal.recv() => {
            print_info!("Received SIGTERM, shutting down");
        }
    }
    
    match state.disconnect_deck().await {
        Ok(_) => {
            print_info!("Screen cleared");
        },
        Err(e) => {
            print_error!("Error while clearing screen: {}", e);
            process::exit(1);
        },
    }
    
    process::exit(0);
}

#[tokio::main]
async fn main() -> process::ExitCode {
    let cli = Cli::parse();
    setup_logging(cli.verbose, cli.no_color);

    let home = PathBuf::from(std::env::var("HOME").unwrap());
    let default_profiles_dir = home.join("Library/Application Support/ajam/profiles");

    match cli.command {
        Command::Run { profiles } => {
            run_listener(&profiles).await;
        },
        Command::Start { profiles } => {
            let profiles = profiles.unwrap_or(default_profiles_dir.display().to_string());
            let bin_path = std::env::current_exe().unwrap();

            let mut arguments = vec![bin_path.display().to_string()];
            if cli.verbose {
                arguments.push("--verbose".to_string());
            }
            arguments.push("run".to_string());
            arguments.push("--profiles".to_string());
            arguments.push(profiles);

            let agent = LaunchAgent {
                label: APP_LABEL.to_string(),
                program_arguments: arguments,
                standard_out_path: "/tmp/ajam.out".to_string(),
                standard_error_path: "/tmp/ajam.err".to_string(),
                keep_alive: true,
                run_at_load: false,
            };

            if let Err(e) = agent.write() {
                print_error!("Failed to write agent: {}", e);
                return process::ExitCode::FAILURE;
            }

            match agent.is_running().await {
                Ok(true) => {
                    print_info!("Agent is already running");
                }
                Ok(false) => {
                    print_info!("Starting agent");
                    if let Err(e) = agent.bootstrap().await {
                        print_error!("Failed to bootstrap agent: {}", e);
                        return process::ExitCode::FAILURE;
                    }
                    print_info!("Agent started");
                }
                Err(e) => {
                    print_error!("Failed to check if agent is running: {}", e);
                    return process::ExitCode::FAILURE;
                }
            }
        },
        Command::Stop => {
            if !LaunchAgent::exists(APP_LABEL) {
                print_error!("Agent does not exist");
                return process::ExitCode::FAILURE;
            }

            let agent = LaunchAgent::from_file(APP_LABEL).unwrap();

            match agent.is_running().await {
                Ok(true) => {
                    print_info!("Stopping agent");
                    if let Err(e) = agent.boot_out().await {
                        print_error!("Failed to stop agent: {}", e);
                        return process::ExitCode::FAILURE;
                    }
                    print_info!("Agent stopped");
                }
                Ok(false) => {
                    print_info!("Agent is not running");
                },
                Err(e) => {
                    print_error!("Failed to check if agent is running: {}", e);
                    return process::ExitCode::FAILURE;
                }
            }
        },
        Command::Status => {
            if !LaunchAgent::exists(APP_LABEL) {
                print_info!("Agent does not exist");
                return process::ExitCode::FAILURE;
            }

            let agent = LaunchAgent::from_file(APP_LABEL).unwrap();
            match agent.is_running().await {
                Ok(true) => {
                    print_info!("Agent is running");
                }
                Ok(false) => {
                    print_info!("Agent is not running");
                }
                Err(e) => {
                    print_error!("Failed to check if agent is running: {}", e);
                    return process::ExitCode::FAILURE;
                }
            }
        },
    }

    process::ExitCode::SUCCESS
}
