use clap::Parser;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Help message for read.
    Start {
        /// The directory containing the profiles
        #[clap(short, long)]
        profiles: Option<String>,
    },
    /// Help message for write.
    Stop,
    Run {
        /// The profile to run
        #[clap(short, long)]
        profiles: String,
    },
    Status,
}

/// Utility to add ticket id to commit message
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Turn debugging information on
    #[arg(short, long)]
    pub verbose: bool,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    #[clap(subcommand)]
    pub command: Command,
}
