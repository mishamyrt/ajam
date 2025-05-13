use clap::Parser;

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

    /// The directory containing the profiles
    #[arg(short, long)]
    pub profiles: String,
}