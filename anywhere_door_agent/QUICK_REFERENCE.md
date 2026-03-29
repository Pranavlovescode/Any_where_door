# Quick Reference - Installation Commands

## Linux (systemd)

### Install with Interactive Menu
```bash
sudo ./scripts/install-systemd.sh
```

### Install with Pre-configured Directories
```bash
sudo ANYWHERE_DOOR_WATCH_ROOTS="/home/user,/var/log,/data" ./scripts/install-systemd.sh
```

### Check Status
```bash
sudo systemctl status anywhere-door-agent.service
```

### View Events
```bash
tail -f /var/log/anywhere-door-agent/file_event_metadata.ndjson
```

### Restart Service
```bash
sudo systemctl restart anywhere-door-agent.service
```

### Edit Configuration
```bash
sudo systemctl edit --full anywhere-door-agent.service
```

### Remove Service
```bash
sudo ./scripts/uninstall.sh
```

---

## Windows (Service Control Manager)

### Install with Interactive Menu
```powershell
# Run PowerShell as Administrator first (Right-click → Run as Administrator)
.\scripts\install-windows-service.ps1
```

### Install with Pre-configured Directories
```powershell
.\scripts\install-windows-service.ps1 -WatchRoots 'C:\Users;D:\Projects' -Recreate
```

### Check Status
```powershell
sc.exe query AnywhereDoorAgent
```

### View Events (Last 20 Lines)
```powershell
Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Tail 20
```

### Watch Events Live
```powershell
Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Wait
```

### Restart Service
```powershell
sc.exe stop AnywhereDoorAgent
sc.exe start AnywhereDoorAgent
```

### Edit Configuration (Registry)
```powershell
# View current config
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters'

# Change watch directories
Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters' -Name 'ANYWHERE_DOOR_WATCH_ROOTS' -Value 'C:\NewPath'
sc.exe stop AnywhereDoorAgent
sc.exe start AnywhereDoorAgent
```

### Remove Service
```powershell
.\scripts\uninstall-windows-service.ps1
```

---

## Path Separator Conventions

### Linux
Comma-separated (`,` character)
```bash
/home/user,/var/log,/data,/opt/sync
```

### Windows
Semicolon-separated (`;` character)
```
C:\Users;D:\Projects;C:\Data;E:\Backup
```

---

## Directory Selection Options

### Option 1: Default/Recommended
- **Linux**: `$HOME` (user's home directory)
- **Windows**: All available drives (auto-detect)

### Option 2: Common Locations
- **Linux**: `/` (entire filesystem - slower)
- **Windows**: `%USERPROFILE%` (user's profile)

### Option 3: Custom Directories
- **Linux**: `/home/user,/var/log,/mnt/data`
- **Windows**: `C:\Users\YourName;D:\Projects;E:\Backups`

### Option 4: Configure Later
- Skip interactive menu
- Edit configuration file after installation
- **Linux**: `sudo systemctl edit --full anywhere-door-agent.service`
- **Windows**: Rerun installer with `-WatchRoots` parameter

---

## Environment Variables (Optional)

Set before installation to skip interactive prompt:

### Linux
```bash
export ANYWHERE_DOOR_WATCH_ROOTS="/path1,/path2"
export ANYWHERE_DOOR_ENABLE_OS_WATCHER=true
export ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT="/custom/output/path/metadata.ndjson"
sudo ./scripts/install-systemd.sh
```

### Windows
```powershell
$env:ANYWHERE_DOOR_WATCH_ROOTS = 'C:\Path1;C:\Path2'
$env:ANYWHERE_DOOR_ENABLE_OS_WATCHER = 'true'
$env:ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT = 'C:\Custom\Output\Path\metadata.ndjson'
.\scripts\install-windows-service.ps1
```

---

## Output Format

Both platforms write to NDJSON (one JSON object per line):

```json
{"timestamp_epoch_ms":1234567890,"event_kind":"create","paths":[{"path":"/home/user/file.txt","exists":true,"is_dir":false,"size_bytes":1024,"modified_epoch_ms":1234567890}]}
```

### Event Kinds
- `create` - New file/directory
- `modify` - File modified
- `remove` - File/directory deleted
- `rename` - Renamed (may be `rename_from` and `rename_to`)
- `access` - File accessed
- `other` - Platform-specific

---

## Common Issues & Solutions

### Linux: Permission Denied
```bash
# Check directory permissions
ls -ld /path/to/watch

# Service needs read+execute permission
sudo chmod g+rx /path/to/watch
```

### Windows: Service Won't Start
```powershell
# Check binary exists
Test-Path "target\release\anywhere_door_agent.exe"

# Rebuild if missing
cargo build --release
```

### No Events Appearing
```bash
# Linux: Check file is in watched directory
stat /path/to/watched/file

# Windows: Check directory permissions
icacls C:\YourWatchDir

# Create test file to verify
touch /tmp/test.txt  # Linux
New-Item C:\Temp\test.txt  # Windows
```

### Service Uses Wrong User (Linux)
```bash
# The service runs as whoever ran installer
# To change: reinstall with different user
sudo ./scripts/uninstall.sh
# Then run installer as different user, then:
sudo ./scripts/install-systemd.sh
```

---

## Files Reference

### Linux
- **Installer**: `scripts/install-systemd.sh`
- **Uninstaller**: `scripts/uninstall.sh`
- **Unit file**: `/etc/systemd/system/anywhere-door-agent.service`
- **Output**: `/var/log/anywhere-door-agent/file_event_metadata.ndjson`

### Windows
- **Installer**: `scripts\install-windows-service.ps1`
- **Uninstaller**: `scripts\uninstall-windows-service.ps1`
- **Registry**: `HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent`
- **Output**: `%APPDATA%\AnywhereDoor\file_event_metadata.ndjson`

---

## Prerequisites

### Linux
- Rust (or pre-built binary)
- systemd (modern Linux distro)
- `sudo` access or root user

### Windows
- Rust (or pre-built binary)
- PowerShell 5.0+
- Administrator user account

---

## Building from Source

Both platforms:
```bash
cargo build --release
```

Binary location:
- **Linux**: `target/release/anywhere_door_agent`
- **Windows**: `target\release\anywhere_door_agent.exe`

---

## Need Help?

1. **Installation guide**: See `deploy/SERVICE_SETUP.md`
2. **Script reference**: See `scripts/README.md`
3. **Windows testing**: See `WINDOWS_TESTING.md`
4. **Implementation details**: See `IMPLEMENTATION_SUMMARY.md`
