use chrono::{Local, NaiveDateTime};
use owo_colors::OwoColorize;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use crate::ssh::create_session;
use crate::steam::get_save_files;
use crate::utils::vprintln;

pub fn upload_to_server(
    game_id: &str,
    local_path: &Path,
    user: &str,
    host: &str,
    excluded_patterns: &Vec<String>,
    verbose: bool,
) {
    let session = create_session(user, host);
    let sftp = session.sftp().unwrap();

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let hostname = if let Ok(h) = hostname::get() {
        h.to_string_lossy().to_string()
    } else {
        String::from("[unknown host]")
    };

    let remote_dir = format!(
        "{}/.better-steam-cloud/{game_id}/{timestamp}-{hostname}",
        dirs::home_dir().unwrap().display()
    );
    sftp.mkdir(Path::new(&remote_dir), 0o755).ok();

    let base = dirs::home_dir().unwrap().join(".better-steam-cloud");
    let _ = sftp.mkdir(&base, 0o700);
    let _ = sftp.mkdir(&base.join(game_id), 0o700);

    let entries = get_save_files(local_path, excluded_patterns);
    let total = entries.iter().filter(|p| p.is_file()).count();

    let mut count = 0;
    for entry in entries {
        let rel = entry.strip_prefix(local_path).unwrap();
        let remote_path = Path::new(&remote_dir).join(rel);

        if entry.is_dir() {
            sftp.mkdir(&remote_path, 0o755).ok();
        } else {
            count += 1;

            print!("\r-> Uploading files for {game_id} - {count}/{total}");
            std::io::stdout().flush().unwrap();
            vprintln(verbose, format!("\nuploading {}", remote_path.display()));

            let mut local_file = File::open(entry).unwrap();
            let mut contents = Vec::new();
            local_file.read_to_end(&mut contents).unwrap();

            let mut remote_file = sftp.create(&remote_path).unwrap();
            remote_file.write_all(&contents).unwrap();
        }
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
            *count += 1;

            print!(
                "\r-> Downloading files for {game_id} - {}/{}{}",
                count,
                total,
                if verbose { "\n" } else { "" }
            );
            std::io::stdout().flush().unwrap();

            vprintln(verbose, format!("downloading {}", remote_path.display()));
            let mut remote_file = sftp.open(&remote_path)?;
            let mut buffer = Vec::new();
            remote_file.read_to_end(&mut buffer)?;

            if let Some(parent) = local_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut file = File::create(&local_path)?;
            file.write_all(&buffer)?;
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

pub fn load_from_server(
    game_id: &str,
    local_path: &Path,
    latest: &bool,
    user: &str,
    host: &str,
    include_sizes: bool,
    verbose: bool,
) {
    let session = create_session(user, host);
    let sftp = session.sftp().unwrap();

    let game_dir = format!(
        "{}/.better-steam-cloud/{game_id}",
        dirs::home_dir().unwrap().display()
    );

    let entries = match sftp.readdir(Path::new(&game_dir)) {
        Ok(e) => e,
        Err(_) => {
            println!(
                "{}",
                format!("No remote save for game with ID {game_id}").red()
            );
            return;
        }
    };

    let mut backups = Vec::new();
    for (path, stat) in entries {
        if stat.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            backups.push(name.to_string());
        }
    }

    backups.sort();
    let chosen = if *latest {
        backups.last().cloned()
    } else {
        println!("Available backups for {game_id}:");
        for (i, name) in backups.iter().enumerate() {
            let remote_dir = Path::new(&game_dir).join(name);

            let (name, hostname) = {
                let mut parts = name.splitn(2, '-');
                (parts.next().unwrap_or(""), parts.next().unwrap_or(""))
            };

            let readable = NaiveDateTime::parse_from_str(name, "%Y%m%d_%H%M%S")
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|_| "invalid timestamp".to_string());

            if include_sizes {
                let size = get_dir_size(&sftp, &remote_dir).unwrap_or(0);
                let size_mb = size as f64 / (1024.0 * 1024.0);
                println!("  [{i}] {readable} on {hostname} ({size_mb:.2} MB)");
            } else {
                println!("  [{i}] {readable} on {hostname}");
            }
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

        println!(
            "{}",
            format!("✓ loadd {game_id} from {backup_name}").green()
        );
    } else {
        println!("No backup found for {game_id}");
    }
}
