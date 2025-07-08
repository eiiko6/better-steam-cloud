mod cli;
use cli::*;

use chrono::Local;
use clap::Parser;
use owo_colors::OwoColorize;
use ssh2::Session;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    let cli = Cli::parse();

    let steam_path = dirs::home_dir()
        .unwrap()
        .join(".local/share/Steam/steamapps/compatdata");
    vprintln(cli.verbose, format!("Steam path: {}", steam_path.display()));

    match &cli.command {
        Command::Save { game_id } => {
            let ids = collect_game_ids(&steam_path, game_id.as_deref());
            let ids_len = ids.len();
            vprintln(
                cli.verbose,
                format!(
                    "Collected {} game ID{}",
                    ids_len,
                    if ids_len == 1 { "" } else { "s" }
                ),
            );

            for id in ids {
                if let Some(path) = get_save_path(&steam_path, &id) {
                    upload_to_server(&id, &path, &cli.user, &cli.host);
                }
            }
        }
        Command::Restore { game_id } => {
            let ids = collect_game_ids(&steam_path, game_id.as_deref());
            let ids_len = ids.len();
            vprintln(
                cli.verbose,
                format!(
                    "Collected {} game ID{}",
                    ids_len,
                    if ids_len == 1 { "" } else { "s" }
                ),
            );

            for id in ids {
                if let Some(_path) = get_save_path(&steam_path, &id) {
                    vprintln(
                        cli.verbose,
                        format!("Restoring save files for game ID {id}"),
                    );
                    println!("Not implemented yet.");
                }
            }
        }
    }
}

fn vprintln(verbose: bool, message: String) {
    if verbose {
        println!("-> {}", message.dimmed());
    }
}

fn collect_game_ids(base: &Path, filter: Option<&str>) -> Vec<String> {
    let entries = fs::read_dir(base).unwrap();
    entries
        .filter_map(|e| {
            let name = e.ok()?.file_name().to_string_lossy().into_owned();
            if filter.is_none_or(|f| f == name) {
                Some(name)
            } else {
                None
            }
        })
        .collect()
}

fn get_save_path(base: &Path, game_id: &str) -> Option<PathBuf> {
    let root = base
        .join(game_id)
        .join("pfx/drive_c/users/steamuser/AppData");
    let save_dirs = ["Local", "LocalLow", "Roaming"];
    for subdir in save_dirs {
        let full = root.join(subdir);
        if full.exists() {
            if let Some(custom) = find_custom_dirs(&full) {
                return Some(custom);
            }
        }
    }
    None
}

fn find_custom_dirs(dir: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        let fname = entry.file_name().into_string().ok()?;
        if fname != "Microsoft" && fname != "Temp" {
            return Some(path);
        }
    }
    None
}

fn upload_to_server(game_id: &str, local_path: &Path, user: &str, host: &str) {
    let tcp = TcpStream::connect(format!("{}:22", host)).unwrap();
    let mut session = Session::new().unwrap();
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();
    session.userauth_agent(user).unwrap();
    let sftp = session.sftp().unwrap();

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let remote_dir = format!(
        "{}/.better-steam-cloud/{game_id}/{timestamp}",
        dirs::home_dir().unwrap().display()
    );
    sftp.mkdir(Path::new(&remote_dir), 0o755).ok();

    // Ensure ~/.better-steam-cloud/<game_id> exists
    let base = dirs::home_dir().unwrap().join(".better-steam-cloud");
    let _ = sftp.mkdir(&base, 0o700);
    let _ = sftp.mkdir(&base.join(game_id), 0o700);

    // Recursively upload files
    for entry in WalkDir::new(local_path).into_iter().filter_map(Result::ok) {
        let rel = entry.path().strip_prefix(local_path).unwrap();
        let remote_path = Path::new(&remote_dir).join(rel);
        if entry.file_type().is_dir() {
            sftp.mkdir(&remote_path, 0o755).ok();
        } else {
            let mut local_file = File::open(entry.path()).unwrap();
            let mut contents = Vec::new();
            local_file.read_to_end(&mut contents).unwrap();

            let mut remote_file = sftp.create(&remote_path).unwrap();
            remote_file.write_all(&contents).unwrap();
        }
    }

    println!(
        "{}",
        format!("✓ Backed up {game_id} to server under timestamp {timestamp}").green()
    );
}
