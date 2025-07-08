mod cli;
use cli::*;

use chrono::{Local, NaiveDateTime};
use clap::Parser;
use owo_colors::OwoColorize;
use ssh2::Session;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    let cli = Cli::parse();

    let steam_path = dirs::home_dir()
        .unwrap()
        .join(".local/share/Steam/steamapps/compatdata");
    println!("Steam path: {}", steam_path.display());

    match &cli.command {
        Command::Save { game_id } => {
            let ids = collect_game_ids(&steam_path, game_id.as_deref(), &cli.ignore);
            let ids_len = ids.len();
            println!(
                "Collected {} game ID{}",
                ids_len,
                if ids_len == 1 { "" } else { "s" }
            );

            for id in ids {
                if let Some(path) = get_save_path(&steam_path, &id) {
                    upload_to_server(&id, &path, &cli.user, &cli.host);
                }
            }
        }
        Command::Restore { game_id, latest } => {
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
                    restore_from_server(&id, &path, latest, &cli.user, &cli.host, cli.verbose);
                }
            }
        }
    }
}

fn vprintln(verbose: bool, message: String) {
    if verbose {
        println!("{}", message.dimmed().bright_black());
    }
}

fn collect_game_ids(base: &Path, filter: Option<&str>, ignore: &[String]) -> Vec<String> {
    let entries = fs::read_dir(base).unwrap();
    entries
        .filter_map(|e| {
            let name = e.ok()?.file_name().to_string_lossy().into_owned();
            if ignore.contains(&name) {
                return None;
            }
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

    let base = dirs::home_dir().unwrap().join(".better-steam-cloud");
    let _ = sftp.mkdir(&base, 0o700);
    let _ = sftp.mkdir(&base.join(game_id), 0o700);

    let entries: Vec<_> = WalkDir::new(local_path)
        .into_iter()
        .filter_map(Result::ok)
        .collect();
    let total = entries.len();

    let mut count = 0;
    for entry in entries {
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

        count += 1;
        print!("\r-> Uploading files for {game_id} - {count}/{total}");
        std::io::stdout().flush().unwrap();
    }

    println!();

    println!(
        "{}",
        format!("✓ Backed up {game_id} to server under timestamp {timestamp}").green()
    );
}

fn download_dir_recursive(
    sftp: &ssh2::Sftp,
    remote: &Path,
    local: &Path,
    verbose: bool,
    total: usize,
    count: &mut usize,
    game_id: &str,
) -> std::io::Result<()> {
    fs::create_dir_all(local)?;

    for (entry_path, stat) in sftp.readdir(remote)? {
        let filename = entry_path.file_name().unwrap();
        let local_path = local.join(filename);
        let remote_path = entry_path;

        if stat.is_dir() {
            download_dir_recursive(
                sftp,
                &remote_path,
                &local_path,
                verbose,
                total,
                count,
                game_id,
            )?;
        } else {
            vprintln(verbose, format!("downloading {}", remote_path.display()));
            let mut remote_file = sftp.open(&remote_path)?;
            let mut buffer = Vec::new();
            remote_file.read_to_end(&mut buffer)?;

            if let Some(parent) = local_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut file = File::create(&local_path)?;
            file.write_all(&buffer)?;

            *count += 1;
            print!("\r-> Downloading files for {game_id} - {}/{}", count, total);
            std::io::stdout().flush().unwrap();
        }
    }

    Ok(())
}

fn count_remote_files(sftp: &ssh2::Sftp, remote: &Path) -> std::io::Result<usize> {
    let mut total = 0;

    for (entry_path, stat) in sftp.readdir(remote)? {
        if stat.is_dir() {
            total += count_remote_files(sftp, &entry_path)?;
        } else {
            total += 1;
        }
    }

    Ok(total)
}

fn get_dir_size(sftp: &ssh2::Sftp, path: &Path) -> std::io::Result<u64> {
    let mut total_size = 0;

    for entry in sftp.readdir(path)? {
        let (entry_path, stat) = entry;
        if stat.is_file() {
            total_size += stat.size.unwrap_or(0);
        } else if stat.is_dir() {
            total_size += get_dir_size(sftp, &entry_path)?;
        }
    }

    Ok(total_size)
}

fn restore_from_server(
    game_id: &str,
    local_path: &Path,
    latest: &bool,
    user: &str,
    host: &str,
    verbose: bool,
) {
    let tcp = TcpStream::connect(format!("{}:22", host)).unwrap();
    let mut session = Session::new().unwrap();
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();
    session.userauth_agent(user).unwrap();
    let sftp = session.sftp().unwrap();

    let game_dir = format!(
        "{}/.better-steam-cloud/{game_id}",
        dirs::home_dir().unwrap().display()
    );

    let entries = sftp.readdir(Path::new(&game_dir)).unwrap();

    let mut backups = vec![];
    for (path, stat) in entries {
        if stat.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                backups.push(name.to_string());
            }
        }
    }

    backups.sort();
    let chosen = if latest.to_owned() {
        backups.last().cloned()
    } else {
        println!("Available backups for {game_id}:");
        for (i, name) in backups.iter().enumerate() {
            let remote_dir = Path::new(&game_dir).join(name);
            let size = get_dir_size(&sftp, &remote_dir).unwrap_or(0);
            let size_mb = size as f64 / (1024.0 * 1024.0);

            let readable = NaiveDateTime::parse_from_str(name, "%Y%m%d_%H%M%S")
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|_| "invalid timestamp".to_string());

            println!("  [{}] {} ({:.2} MB)", i, readable, size_mb);
        }

        print!("Pick a backup index: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim().parse::<usize>() {
            Ok(index) if index < backups.len() => Some(backups[index].clone()),
            _ => {
                eprintln!("Invalid index.");
                return;
            }
        }
    };

    if let Some(backup_name) = chosen {
        let remote_dir = Path::new(&game_dir).join(&backup_name);
        println!("Using backup {backup_name}");

        println!("Backing up current local save first...");
        upload_to_server(&format!("{game_id}_pre_restore"), local_path, user, host);

        let total = count_remote_files(&sftp, &remote_dir).unwrap();
        let mut count = 0;
        download_dir_recursive(
            &sftp,
            &remote_dir,
            local_path,
            verbose,
            total,
            &mut count,
            game_id,
        )
        .unwrap();
        println!();

        println!("✓ Restored {game_id} from {backup_name}");
    } else {
        println!("No backup found for {game_id}");
    }
}
