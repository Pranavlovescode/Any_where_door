param(
    [string]$ServiceName = "AnywhereDoorAgent",
    [string]$DisplayName = "Anywhere Door Agent",
    [string]$Description = "Anywhere Door background agent - OS Level File Watcher (Rust)",
    [string]$ExePath = "$PSScriptRoot\..\target\release\anywhere_door_agent.exe",
    [switch]$Recreate,
    [string]$WatchRoots = ""
)

# Require administrator privileges
$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    throw "ERROR: Run this script in an elevated PowerShell session (Run as Administrator)."
}

Write-Host "=== Installing Anywhere Door Agent ===" -ForegroundColor Green
Write-Host ""

# Verify binary exists
if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR: Rust binary not found at: $ExePath" -ForegroundColor Red
    Write-Host "Build first with: cargo build --release" -ForegroundColor Yellow
    exit 1
}

$resolvedExe = (Resolve-Path $ExePath).Path
Write-Host "Binary location: $resolvedExe"
Write-Host ""

# Get or prompt for watch directories
if (-not $WatchRoots) {
    Write-Host "=== Directory Selection ===" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Choose which directories to watch:" -ForegroundColor Yellow
    Write-Host "[1] All available drives (C:\, D:\, etc.) - Default"
    Write-Host "[2] User profile directory ($env:USERPROFILE)"
    Write-Host "[3] Custom directories (enter paths separated by semicolon)"
    Write-Host "[4] Skip directory selection (configure manually later)"
    Write-Host ""
    
    $choice = Read-Host "Enter choice (1-4)"
    
    switch ($choice) {
        "1" {
            Write-Host "Selected: All drives" -ForegroundColor Green
            $WatchRoots = ""  # Empty means auto-detect all drives
        }
        "2" {
            $WatchRoots = $env:USERPROFILE
            Write-Host "Selected: $WatchRoots" -ForegroundColor Green
        }
        "3" {
            Write-Host ""
            Write-Host "Enter directories to watch (separate with semicolon):" -ForegroundColor Yellow
            Write-Host "Example: C:\Users\YourName;D:\Projects;C:\Data" -ForegroundColor Gray
            $WatchRoots = Read-Host "Directories"
            Write-Host "Selected: $WatchRoots" -ForegroundColor Green
        }
        "4" {
            Write-Host "Skipping directory selection. Configure ANYWHERE_DOOR_WATCH_ROOTS manually later." -ForegroundColor Yellow
            $WatchRoots = ""
        }
        default {
            Write-Host "Invalid choice. Using default (all drives)." -ForegroundColor Yellow
            $WatchRoots = ""
        }
    }
}

Write-Host ""
Write-Host "[1/4] Checking for existing service..." -ForegroundColor Cyan

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if ($Recreate -and $existing) {
    Write-Host "Removing existing service..." -ForegroundColor Yellow
    sc.exe stop $ServiceName 2>&1 | Out-Null
    Start-Sleep -Seconds 1
    sc.exe delete $ServiceName 2>&1 | Out-Null
    Start-Sleep -Seconds 1
    $existing = $null
    Write-Host "✓ Existing service removed" -ForegroundColor Green
}

if (-not $existing) {
    Write-Host "✓ No existing service found, will create new one" -ForegroundColor Green
} else {
    Write-Host "✓ Existing service found, will update configuration" -ForegroundColor Green
}

Write-Host ""
Write-Host "[2/4] Creating/Updating Windows service..." -ForegroundColor Cyan

$binPath = '"' + $resolvedExe + '" --windows-service'

if (-not $existing) {
    sc.exe create $ServiceName binPath= $binPath start= auto DisplayName= "$DisplayName" 2>&1 | Out-Null
    sc.exe description $ServiceName "$Description" 2>&1 | Out-Null
} else {
    sc.exe stop $ServiceName 2>&1 | Out-Null
    Start-Sleep -Seconds 1
    sc.exe config $ServiceName binPath= $binPath start= auto DisplayName= "$DisplayName" 2>&1 | Out-Null
    sc.exe description $ServiceName "$Description" 2>&1 | Out-Null
}

Write-Host "✓ Service created/updated" -ForegroundColor Green

# Configure watch roots via environment variables (stored in registry for service)
Write-Host ""
Write-Host "[3/4] Configuring watch directories..." -ForegroundColor Cyan

if ($WatchRoots) {
    # Set environment variable in registry so service can access it
    # Services read environment from: HKLM\SYSTEM\CurrentControlSet\Services\<ServiceName>\Parameters
    $regPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$ServiceName"
    
    if (-not (Test-Path $regPath)) {
        New-Item -Path $regPath -Force | Out-Null
    }
    
    # Create Parameters key if it doesn't exist
    $paramsPath = "$regPath\Parameters"
    if (-not (Test-Path $paramsPath)) {
        New-Item -Path $paramsPath -Force | Out-Null
    }
    
    Set-ItemProperty -Path $paramsPath -Name "ANYWHERE_DOOR_WATCH_ROOTS" -Value $WatchRoots -Force
    Write-Host "Watch directories: $WatchRoots" -ForegroundColor Green
} else {
    Write-Host "Watch directories: Auto-detect all drives (default)" -ForegroundColor Green
}

# Enable watcher
$paramsPath = "$regPath\Parameters"
if (-not (Test-Path $paramsPath)) {
    New-Item -Path $paramsPath -Force | Out-Null
}
Set-ItemProperty -Path $paramsPath -Name "ANYWHERE_DOOR_ENABLE_OS_WATCHER" -Value "true" -Force
Set-ItemProperty -Path $paramsPath -Name "ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT" -Value "%APPDATA%\AnywhereDoor\file_event_metadata.ndjson" -Force

# Create output directory
$outputDir = "$env:APPDATA\AnywhereDoor"
if (-not (Test-Path $outputDir)) {
    New-Item -Path $outputDir -ItemType Directory -Force | Out-Null
}

Write-Host "✓ Watch directories configured" -ForegroundColor Green

Write-Host ""
Write-Host "[4/4] Starting service..." -ForegroundColor Cyan

sc.exe stop $ServiceName 2>&1 | Out-Null
Start-Sleep -Seconds 2
sc.exe start $ServiceName 2>&1 | Out-Null
Start-Sleep -Seconds 2

$status = sc.exe query $ServiceName
Write-Host "✓ Service started" -ForegroundColor Green

Write-Host ""
Write-Host "=== Installation Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Configuration:" -ForegroundColor Yellow
Write-Host "  Service name: $ServiceName"
Write-Host "  Display name: $DisplayName"
Write-Host "  Binary: $resolvedExe"
Write-Host "  Output directory: $outputDir"
if ($WatchRoots) {
    Write-Host "  Watch directories: $WatchRoots"
} else {
    Write-Host "  Watch directories: All available drives (auto-detected)"
}
Write-Host ""
Write-Host "Useful commands:" -ForegroundColor Yellow
Write-Host "  Get status:     sc.exe query $ServiceName"
Write-Host "  Start service:  sc.exe start $ServiceName"
Write-Host "  Stop service:   sc.exe stop $ServiceName"
Write-Host "  View logs:      Get-Content $outputDir\file_event_metadata.ndjson -Tail 20"
Write-Host ""
Write-Host "To change watch directories later:" -ForegroundColor Cyan
Write-Host "  .\scripts\install-windows-service.ps1 -WatchRoots 'C:\Users;D:\Projects' -Recreate"
Write-Host ""
Write-Host "Current service status:" -ForegroundColor Cyan
Write-Host $status
