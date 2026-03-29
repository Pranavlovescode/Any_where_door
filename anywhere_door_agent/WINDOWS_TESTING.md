# Testing Anywhere Door Agent on Windows

This guide walks through testing the service installation on Windows machines.

## Prerequisites

- Windows 10, 11, or Windows Server 2019+
- PowerShell 5.0+ (built-in on modern Windows)
- Administrator access required
- Administrator PowerShell console open

## Building the Windows Binary

### Step 1: Install Rust (if not already installed)

```powershell
# Download from https://rustup.rs/
# Run the installer and follow prompts
rustup update
```

### Step 2: Build Release Binary

```powershell
cd C:\path\to\Any_where_door\anywhere_door_agent
cargo build --release
```

Binary will be at: `target\release\anywhere_door_agent.exe`

## Testing Installation

### Test 1: Interactive Directory Selection - Option 1 (Home Directory)

```powershell
# Open PowerShell as Administrator
.\scripts\install-windows-service.ps1

# When prompted:
# Enter choice (1-4): 1
# Should output: [GREEN] Selected: All drives
```

**Expected results:**
- Service created: `AnywhereDoorAgent`
- Service status: Running
- Output directory: `%APPDATA%\AnywhereDoor\` created
- File: `file_event_metadata.ndjson` started being written

**Verify with:**
```powershell
sc.exe query AnywhereDoorAgent
dir $env:APPDATA\AnywhereDoor\
Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Tail 5
```

### Test 2: Interactive Directory Selection - Option 2 (User Profile)

```powershell
.\scripts\uninstall-windows-service.ps1
# When prompted: yes

# Reinstall with option 2
.\scripts\install-windows-service.ps1

# When prompted:
# Enter choice (1-4): 2
# Should output: [GREEN] Selected: C:\Users\YourName
```

**Verify:**
```powershell
# Check registry to confirm path
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters' | Select-Object -Property ANYWHERE_DOOR_WATCH_ROOTS
# Should show: C:\Users\YourName (or similar)
```

### Test 3: Interactive Directory Selection - Option 3 (Custom Paths)

```powershell
.\scripts\uninstall-windows-service.ps1

# Reinstall with option 3
.\scripts\install-windows-service.ps1

# When prompted:
# Enter choice (1-4): 3
# Enter directories: C:\Users;D:\Projects;C:\Documents
# Should output: [GREEN] Selected: C:\Users;D:\Projects;C:\Documents
```

**Verify registry:**
```powershell
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters' | Select-Object -Property ANYWHERE_DOOR_WATCH_ROOTS
# Should show: C:\Users;D:\Projects;C:\Documents
```

### Test 4: Command-Line Parameter

```powershell
.\scripts\uninstall-windows-service.ps1

# Install with explicit directories (no prompt)
.\scripts\install-windows-service.ps1 -WatchRoots 'C:\Windows\Temp;C:\Users' -Recreate
```

**Expected:**
- No interactive prompts
- Service created with specified directories
- Registry shows: `C:\Windows\Temp;C:\Users`

### Test 5: File Event Monitoring

```powershell
# After installation, create test files
New-Item "C:\Users\$env:USERNAME\test_file.txt" -ItemType File
Start-Sleep -Seconds 2

# Check output file for events
Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Tail 10

# Should show JSON events like:
# {"timestamp_epoch_ms":1234567890,"event_kind":"create",...}
```

### Test 6: Service Management

```powershell
# Stop service
sc.exe stop AnywhereDoorAgent

# Verify stopped
sc.exe query AnywhereDoorAgent
# Should show: STATE: 1 STOPPED

# Start service
sc.exe start AnywhereDoorAgent

# Verify running
sc.exe query AnywhereDoorAgent
# Should show: STATE: 4 RUNNING

# Create file to verify events are captured
New-Item "C:\Users\$env:USERNAME\test2.txt" -ItemType File
Start-Sleep -Seconds 1
Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Tail 1
```

### Test 7: Uninstallation

```powershell
.\scripts\uninstall-windows-service.ps1

# Verify service removed
sc.exe query AnywhereDoorAgent
# Should show: [SC] EnumServiceStatus: The specified service does not exist as an installed service.

# Verify registry cleaned
Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters' -ErrorAction SilentlyContinue
# Should return nothing
```

## Verification Checklist

### Installation
- [ ] PowerShell runs as Administrator without error
- [ ] Interactive menu displays correctly
- [ ] Selection is registered and echoed back
- [ ] Service is created and running
- [ ] Output directory exists: `%APPDATA%\AnywhereDoor\`
- [ ] Registry configured: `HKLM:\...\Parameters\ANYWHERE_DOOR_WATCH_ROOTS`

### Operation
- [ ] Creating files generates events in metadata.ndjson
- [ ] Events are valid JSON format (one per line)
- [ ] Timestamps are in epoch milliseconds
- [ ] Event kinds (create, modify, etc.) are correct
- [ ] File metadata includes path, size, modification time

### Configuration
- [ ] Custom directories are correctly stored
- [ ] Semicolon separator works for multiple paths
- [ ] Re-running installer with `-Recreate` works
- [ ] Manually edited registry paths are honored after restart

### Uninstallation
- [ ] Service stops cleanly
- [ ] Service is removed from registry
- [ ] No error messages
- [ ] Can reinstall afterwards

## Troubleshooting Windows Tests

### "Run this script in an elevated PowerShell session"
- Right-click PowerShell application
- Select "Run as Administrator"
- Run script again

### Service remains in "STOPPING" state
```powershell
# Force kill the process
Stop-Service -Name AnywhereDoorAgent -Force
Remove-Service -Name AnywhereDoorAgent
```

### Output file not created
- Check output directory created: `dir $env:APPDATA\AnywhereDoor\`
- Check service is running: `sc.exe query AnywhereDoorAgent`
- Check registry has write path: `Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent\Parameters'`
- Create test file in watched directory to trigger event

### Registry entries not updating
```powershell
# After modifying registry, restart service
sc.exe stop AnywhereDoorAgent
Start-Sleep -Seconds 2
sc.exe start AnywhereDoorAgent
Start-Sleep -Seconds 2
```

### "Rust binary not found" error
```powershell
# Ensure binary exists
Test-Path "target\release\anywhere_door_agent.exe"

# If not, build it
cargo build --release

# Or specify explicit path
.\scripts\install-windows-service.ps1 -ExePath "C:\full\path\to\anywhere_door_agent.exe"
```

## Real-World Scenarios

### Scenario 1: Monitor Documents Folder Only

```powershell
.\scripts\install-windows-service.ps1 -WatchRoots "$env:USERPROFILE\Documents" -Recreate
```

### Scenario 2: Monitor Multiple Cloud Storage Folders

```powershell
$paths = "$env:USERPROFILE\OneDrive;$env:USERPROFILE\Google Drive;$env:USERPROFILE\Dropbox"
.\scripts\install-windows-service.ps1 -WatchRoots $paths -Recreate
```

### Scenario 3: Monitor External Drive

```powershell
# Assuming E: is an external USB drive
.\scripts\install-windows-service.ps1 -WatchRoots "E:\" -Recreate
```

### Scenario 4: System-Wide Monitoring (Requires caution)

```powershell
# Monitor multiple system locations
$paths = "C:\Program Files\*;C:\Users;C:\Windows\Temp"
.\scripts\install-windows-service.ps1 -WatchRoots $paths -Recreate

# Note: This may miss events due to access restrictions on C:\Windows
```

## Performance Testing

### High Volume File Creation

```powershell
# Create many test files
foreach ($i in 1..100) {
    New-Item "C:\Users\$env:USERNAME\test_$i.txt" -ItemType File | Out-Null
}

# Check event count
(Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson).Count
# Should be significantly more than 100 (due to multiple events per file)
```

### Large File Modification

```powershell
# Create a large file
$launchPad = New-Object -TypeName System.Collections.Generic.List[string]
for ($i = 0; $i -lt 10000; $i++) {
    $launchPad.Add("Line $i with some test data")
}
$launchPad | Out-File -FilePath "C:\Users\$env:USERNAME\large_file.txt"

# Check for events
Get-Content $env:APPDATA\AnywhereDoor\file_event_metadata.ndjson -Tail 5
```

## Debugging

### View Live Service Output (Windows Event Viewer)

```powershell
# Open Event Viewer
eventvwr.msc

# Navigate to: Windows Logs > System
# Look for events from: AnywhereDoorAgent or Service Control Manager
```

### Enable Detailed PowerShell Logging (if needed)

```powershell
$ExecutionContext.SessionState.LanguageMode = "FullLanguage"
.\scripts\install-windows-service.ps1 -Debug
```

### Check Binary Information

```powershell
# Verify it's the right binary
Get-ChildItem "target\release\anywhere_door_agent.exe" -Force

# Check version/date (if you're building multiple times)
Get-ChildItem "target\release\*" -Include "anywhere_door_agent*" | Select-Object Name, LastWriteTime, Length
```

## Cross-Platform Testing

After testing on Windows, verify the cross-platform watcher works:

1. **On Windows**: Create json events with semicolon-separated paths: `C:\Users;D:\Data`
2. **On Linux**: Create event with comma-separated paths: `/home/user,/var/log`
3. **Verify**: Both output same NDJSON format

The watcher.rs file should handle both separator styles seamlessly.
