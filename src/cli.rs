use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// SSH username
    pub user: String,

    /// SSH hostname or IP
    pub host: String,

    #[command(subcommand)]
    pub command: Command,

    #[arg(short, long)]
    pub verbose: bool,

    /// List of game IDs to ignore
    #[arg(short, long)]
    pub ignore: Vec<String>,
}

#[derive(Subcommand)]
pub enum Command {
    Save {
        #[arg(short, long)]
        game_id: Option<String>,
    },
    Restore {
        #[arg(short, long)]
        game_id: Option<String>,
    },
}
