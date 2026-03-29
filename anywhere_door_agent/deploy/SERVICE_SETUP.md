# Anywhere Door Agent Service Setup

This project provides cross-platform service installation for:
- **Linux**: systemd (with interactive directory selection)
- **Windows**: Windows Service Control Manager (with interactive directory selection)

Both installers support choosing which directories to monitor at installation time.

---

## Linux (systemd)

### Quick Start

1. **Build release binary:**
   ```bash
   cargo build --release
   ```

2. **Install and start service:**
   ```bash
   sudo ./scripts/install-systemd.sh
   ```
   
   The script will prompt you to select watch directories:
   - Option 1: Home directory (default)
   - Option 2: Entire filesystem (/)
   - Option 3: Custom directories (comma-separated)
   - Option 4: Skip and configure manually

3. **Verify installation:**
   ```bash
   systemctl status anywhere-door-agent.service
   tail -f /var/log/anywhere-door-agent/file_event_metadata.ndjson
   ```

### Service Management

```bash
# Check status
sudo systemctl status anywhere-door-agent.service

# Restart service
sudo systemctl restart anywhere-door-agent.service

# Stop service
sudo systemctl stop anywhere-door-agent.service

# Disable auto-start
sudo systemctl disable anywhere-door-agent.service

# View live logs (last 50 lines)
tail -f /var/log/anywhere-door-agent/file_event_metadata.ndjson
```

### Configuration Files

- **Unit file**: `/etc/systemd/system/anywhere-door-agent.service`
- **Output file**: `/var/log/anywhere-door-agent/file_event_metadata.ndjson`

### Changing Watch Directories (Linux)

Edit the systemd unit file:
```bash
sudo systemctl edit --full anywhere-door-agent.service
```

Modify the `Environment=ANYWHERE_DOOR_WATCH_ROOTS` line:
```ini
# Watch single directory
Environment=ANYWHERE_DOOR_WATCH_ROOTS=/home/user

# Watch multiple directories (comma-separated)
Environment=ANYWHERE_DOOR_WATCH_ROOTS=/home/user,/var/log,/opt/data
```

Then reload and restart:
```bash
sudo systemctl daemon-reload
sudo systemctl restart anywhere-door-agent.service
```

### Uninstalling (Linux)

```bash
sudo ./scripts/uninstall.sh
```

---

## Windows (Service Control Manager)

### Quick Start

1. **Build release binary:**
   ```powershell
   cargo build --release
   ```

2. **Open PowerShell as Administrator** (Right-click → Run as Administrator)

3. **Install and start service:**
   ```powershell
   .\scripts\install-windows-service.ps1
   ```
   
   The script will prompt you to select watch directories:
   - Option 1: All available drives (C:, D:, etc.) - Default
   - Option 2: User profile directory (%USERPROFILE%)
   - Option 3: Custom directories (semicolon-separated)
   - Option 4: Skip and configure manually

4. **Verify installation:**
   ```powershell
   sc.exe query AnywhereDoorAgent
   Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Tail 20
   ```

### Service Management

```powershell
# Check status
sc.exe query AnywhereDoorAgent

# Start service
sc.exe start AnywhereDoorAgent

# Stop service
sc.exe stop AnywhereDoorAgent

# View logs (last 20 lines)
Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Tail 20

# Watch logs in real-time
Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Wait
```

### Configuration

Windows stores service environment variables in the registry:
```
HKLM\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters
```

You can view/edit them with PowerShell:
```powershell
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters'
```

### Changing Watch Directories (Windows)

Reinstall with different directories:
```powershell
.\scripts\install-windows-service.ps1 -WatchRoots 'C:\Users;D:\Projects;E:\Data' -Recreate
```

Or manually edit registry:
```powershell
$regPath = 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters'
Set-ItemProperty -Path $regPath -Name 'ANYWHERE_DOOR_WATCH_ROOTS' -Value 'C:\Users;D:\Data'

# Then restart service
sc.exe stop AnywhereDoorAgent
sc.exe start AnywhereDoorAgent
```

### Uninstalling (Windows)

Open PowerShell as Administrator:
```powershell
.\scripts\uninstall-windows-service.ps1
```

### Output Location

- **Output directory**: `%APPDATA%\AnywhereDoor\` (usually `C:\Users\YourName\AppData\Roaming\AnywhereDoor\`)
- **Output file**: `file_event_metadata.ndjson`

---

## Cross-Platform Features

### Directory Selection

Both installers support interactive directory selection:
- **Linux uses comma separator**: `/home/user,/var/log,/data`
- **Windows uses semicolon separator**: `C:\Users;D:\Projects;C:\Data`
- The code automatically detects which separator is used

### Output Format (NDJSON)

Both platforms output filesystem events in the same JSON format:
```json
{"timestamp_epoch_ms":1234567890,"event_kind":"create","paths":[{"path":"/home/user/file.txt","exists":true,"is_dir":false,"size_bytes":1024,"modified_epoch_ms":1234567890}]}
{"timestamp_epoch_ms":1234567891,"event_kind":"modify","paths":[{"path":"/home/user/file.txt","exists":true,"is_dir":false,"size_bytes":2048,"modified_epoch_ms":1234567891}]}
```

Event kinds:
- `create` - New file/directory created
- `modify` - File content or metadata modified
- `remove` - File/directory deleted
- `rename` - File/directory renamed (may emit both `rename_from` and `rename_to`)
- `access` - File accessed (may vary by OS)
- `other` - Platform-specific events

### Environment Variables

You can override configuration via environment variables:

**Linux** (all OS):
```bash
export ANYWHERE_DOOR_WATCH_ROOTS="/path1,/path2"
export ANYWHERE_DOOR_ENABLE_OS_WATCHER=true
export ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT="/custom/path/metadata.ndjson"
```

**Windows**:
```powershell
$env:ANYWHERE_DOOR_WATCH_ROOTS = 'C:\Path1;C:\Path2'
$env:ANYWHERE_DOOR_ENABLE_OS_WATCHER = 'true'
$env:ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT = 'C:\Custom\Path\metadata.ndjson'
```

---

## Troubleshooting

### Linux: Permission Denied

If the watcher can't access a directory:
1. Verify the user running the service has read permissions: `ls -ld /path/to/dir`
2. The service runs as the user who installed it
3. Change directories in the systemd unit file and restart

### Windows: Service Won't Start

1. Verify PowerShell ran as Administrator during installation
2. Check Event Viewer → Windows Logs → System for error details
3. Ensure the binary is at: `target\release\anywhere_door_agent.exe`
4. Try reinstalling with `-Recreate` flag:
   ```powershell
   .\scripts\install-windows-service.ps1 -Recreate
   ```

### High CPU Usage

Both platforms use OS-level file watching APIs (inotify on Linux, ReadDirectoryChangesW on Windows).
If experiencing high CPU:
1. Check if too many directories are being watched
2. Consider watching specific subdirectories instead of `/` or `C:\`
3. Reduce the number of directories in `ANYWHERE_DOOR_WATCH_ROOTS`

---

## Advanced: Manual Configuration

### Linux - Edit systemd unit directly

```bash
sudo nano /etc/systemd/system/anywhere-door-agent.service
```

Key variables:
```ini
[Service]
User=deilsy
Group=deilsy
Environment=ANYWHERE_DOOR_WATCH_ROOTS=/home/deilsy
Environment=ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT=/var/log/anywhere-door-agent/file_event_metadata.ndjson
```

### Windows - Registry Editor

Press `Win+R`, type `regedit`, navigate to:
```
HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters
```

Edit string values for environment variables.

---

## Files Reference

- **Linux installer**: `scripts/install-systemd.sh`
- **Linux uninstaller**: `scripts/uninstall.sh`
- **Linux unit file**: `deploy/linux/anywhere-door-agent.service`
- **Windows installer**: `scripts/install-windows-service.ps1`
- **Windows uninstaller**: `scripts/uninstall-windows-service.ps1`
- **Watcher code**: `src/filesystem/watcher.rs`
