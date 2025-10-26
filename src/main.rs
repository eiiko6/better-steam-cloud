mod backup;
mod cli;
mod ssh;
mod steam;
mod utils;

use clap::Parser;
use steam::{collect_game_ids, get_save_path};

use backup::{restore_from_server, upload_to_server};
use cli::*;
use utils::vprintln;

fn main() {
    let cli = Cli::parse();

    let steam_path = dirs::home_dir()
        .unwrap()
        .join(".local/share/Steam/steamapps/compatdata");
    vprintln(cli.verbose, format!("Steam path: {}", steam_path.display()));

    match &cli.command {
        Command::Save {
            game_id,
            exclude_patterns,
        } => {
            let ids = collect_game_ids(&steam_path, game_id.as_deref(), &cli.ignore);
            let ids_len = ids.len();
            println!(
                "Collected {} game ID{}",
                ids_len,
                if ids_len == 1 { "" } else { "s" }
            );

            for id in ids {
                if let Some(path) = get_save_path(&steam_path, &id) {
                    upload_to_server(
                        &id,
                        &path,
                        &cli.user,
                        &cli.host,
                        exclude_patterns,
                        cli.verbose,
                    );
                }
            }
        }
        Command::Restore {
            game_id,
            latest,
            hide_sizes,
        } => {
            let ids = collect_game_ids(&steam_path, game_id.as_deref(), &cli.ignore);
            let ids_len = ids.len();
            println!(
                "Collected {} game ID{}",
                ids_len,
                if ids_len == 1 { "" } else { "s" }
            );

            for id in ids {
                if let Some(path) = get_save_path(&steam_path, &id) {
                    println!("Restoring save files for game ID {id}...");
                    restore_from_server(
                        &id,
                        &path,
                        latest,
                        &cli.user,
                        &cli.host,
                        !*hide_sizes,
                        cli.verbose,
                    );
                }
            }
        }
    }
}
