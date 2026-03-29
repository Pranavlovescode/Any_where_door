# Installation Scripts

This directory contains service installation and management scripts for cross-platform deployment.

## Linux

### `install-systemd.sh`
Interactive installer for Linux systemd service.

**Features:**
- Auto-detects calling user and runs service as that user
- Interactive directory selection:
  - Home directory (default)
  - Entire filesystem
  - Custom directories (comma-separated)
- Handles permissions automatically
- Sets up systemd unit file with environment variables
- Creates log directories

**Usage:**
```bash
sudo ./scripts/install-systemd.sh
```

**Optional environment variable override:**
```bash
sudo ANYWHERE_DOOR_WATCH_ROOTS=/custom/path ./scripts/install-systemd.sh
```

### `uninstall.sh`
Removes the systemd service completely.

**Features:**
- Stops the running service
- Disables autostart
- Deletes service unit file
- Removes system user (if created)
- Cleans up log and working directories

**Usage:**
```bash
sudo ./scripts/uninstall.sh
```

---

## Windows

### `install-windows-service.ps1`
Interactive installer for Windows Service Control Manager.

**Features:**
- Requires Administrator privileges (enforced)
- Interactive directory selection:
  - All available drives (auto-detect)
  - User profile directory
  - Custom directories (semicolon-separated)
- Stores configuration in Windows registry
- Creates output directory in `%APPDATA%\AnywhereDoor\`
- Handles service creation/update
- Optional: Recreate option to clean install

**Usage (elevated PowerShell):**
```powershell
.\scripts\install-windows-service.ps1
```

**With explicit watch directories:**
```powershell
.\scripts\install-windows-service.ps1 -WatchRoots 'C:\Users;D:\Projects' -Recreate
```

**Parameters:**
- `-ServiceName`: Service name (default: `AnywhereDoorAgent`)
- `-DisplayName`: Display name in Services (default: `Anywhere Door Agent`)
- `-Description`: Service description
- `-ExePath`: Path to binary (auto-detected relative to script)
- `-WatchRoots`: Directories to watch (semicolon-separated)
- `-Recreate`: Force clean reinstall (stops, deletes, recreates)

### `uninstall-windows-service.ps1`
Removes the Windows service completely.

**Features:**
- Requires Administrator privileges
- Stops the running service
- Deletes service from registry
- Cleans up registry entries
- Preserves data files (can be manually deleted)

**Usage (elevated PowerShell):**
```powershell
.\scripts\uninstall-windows-service.ps1
```

---

## Deprecated

### `register-windows-service.ps1` (Old)
⚠️ Superseded by `install-windows-service.ps1`

This old script lacks:
- Directory selection prompts
- Configuration management
- Interactive user guidance

Users should migrate to `install-windows-service.ps1`.

---

## Directory Selection Feature

### Interactive Prompts

Both Linux and Windows installers provide interactive menus:

```
=== Directory Selection ===

Choose which directories to watch:
[1] Home directory (/home/user) - Default
[2] Entire filesystem (/)
[3] Custom directories (enter paths separated by comma)
[4] Skip directory selection (configure manually later)

Enter choice (1-4):
```

### Path Separator Conventions

- **Linux**: Comma-separated (`,`)
  - Example: `/home/user,/var/log,/opt/data`

- **Windows**: Semicolon-separated (`;`)
  - Example: `C:\Users;D:\Projects;C:\Data`

The watcher code automatically detects which separator is used.

---

## Configuration Files

After installation, configuration is stored in:

**Linux:**
- Systemd unit: `/etc/systemd/system/anywhere-door-agent.service`
- Edit with: `sudo systemctl edit --full anywhere-door-agent.service`

**Windows:**
- Registry path: `HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters`
- Edit with: PowerShell `Set-ItemProperty` or Registry Editor

---

## Building Before Installation

Both installers require the release binary to exist:

```bash
cargo build --release
```

This creates:
- Linux: `target/release/anywhere_door_agent`
- Windows: `target/release/anywhere_door_agent.exe`

If the binary doesn't exist, installers will exit with an error.

---

## Troubleshooting

### Linux: "systemctl not found"
- Your system doesn't use systemd (older Linux or non-standard distro)
- Install systemd or use manual service setup

### Windows: "Run this script in an elevated PowerShell session"
- Right-click PowerShell → Run as Administrator
- Then run the installer

### Linux: Permission Denied
- Must use `sudo` to create system directories
- Service will run as the user who ran the installer

### Windows: "Rust binary not found"
- Run `cargo build --release` first
- Verify binary exists at: `target\release\anywhere_door_agent.exe`

---

## Advanced: Reconfiguration

### Linux - Change watch directories
```bash
sudo systemctl edit --full anywhere-door-agent.service
# Modify ANYWHERE_DOOR_WATCH_ROOTS line
sudo systemctl daemon-reload
sudo systemctl restart anywhere-door-agent.service
```

### Windows - Reinstall with new directories
```powershell
.\scripts\install-windows-service.ps1 -WatchRoots 'C:\NewPath1;C:\NewPath2' -Recreate
```

---

## Service Lifecycle

1. **Pre-Installation**: Verify binary exists, check permissions
2. **Installation**: Create service, configure directories, set environment variables
3. **Startup**: Service starts automatically on boot, loads watch directories
4. **Runtime**: Watches configured directories, outputs events to NDJSON file
5. **Uninstall**: Stop service, remove registry/systemd entries, clean directories

---

## Notes

- Service runs as the **calling user** on Linux (no separate service user created)
- Service runs with **normal privileges** on Windows (no SYSTEM account needed)
- Watch directories are **recursive** - subdirectories are automatically monitored
- Output format is **NDJSON** - each event is a single JSON line, machine-parseable
- No firewall configuration needed - all operations are local
