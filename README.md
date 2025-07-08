# Better Steam Cloud

A simple CLI utility to **backup** and **restore** Steam games save data over SSH.
Meant as a replacement to **Steam Cloud** for those who have a personal server at hand

I was tired of steam cloud deleting my save data, so I made my own with rust and ssh.

## ✨ Features

- [x] Save or restore game saves
- [x] Target specific games by ID
- [x] Ignore selected games
- [x] Restore latest save automatically
- [x] Verbose mode
- [ ] Configuration file

## 🔧 Usage

```bash
bsc [OPTIONS] <USER> <HOST> <COMMAND> [COMMAND OPTIONS]
```

### Examples

- Backup all saves, ignoring some games:

  ```bash
  bsc alice 192.168.1.10 -i 730 -i 440 save
  ```

- Backup a specific game:

  ```bash
  bsc alice 192.168.1.10 save --game-id 1657630
  ```

- Restore the latest backup of all games:

  ```bash
  bsc alice 192.168.1.10 restore --latest
  ```

- Restore a specific game:

  ```bash
  bsc alice 192.168.1.10 restore --game-id 1657630
  ```

### Quick flags

- `-v`, `--verbose`: verbose output
- `-i <ID>`, `--ignore <ID>`: ignore game by ID (can be repeated)
- `-g <ID>`, `--game-id <ID>`: target a specific game
- `-l`, `--latest`: restore latest backup

## 🔐 SSH Requirement

Ensure key-based SSH access is set up between your machine and the backup host.  
We use the SSH agent (ssh-agent or gpg-agent) to authenticate using your loaded private key. Ensure the `SSH_AUTH_SOCK` environment variable is set.

For fish:

```fish
eval (ssh-agent -c)
ssh-add ~/.ssh/id_rsa
```
