param(
    [string]$ServiceName = "AnywhereDoorAgent",
    [string]$DisplayName = "Anywhere Door Agent",
    [string]$Description = "Anywhere Door background agent - OS Level File Watcher (Rust)",
    [string]$ExePath = "$PSScriptRoot\..\target\release\anywhere_door_agent.exe",
    [switch]$Recreate,
    [string]$WatchRoots = "",
    [string]$ServerUrl = ""
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

# ============================================================================
# STEP 0: AUTHENTICATION & DEVICE REGISTRATION
# ============================================================================

$credentialsFile = "$env:USERPROFILE\.anywheredoor"
$watchConfigFile = "$env:USERPROFILE\.anywheredoor_watch_roots"

if (-not $ServerUrl) {
    if ($env:ANYWHERE_DOOR_SERVER_URL) {
        $ServerUrl = $env:ANYWHERE_DOOR_SERVER_URL
    } else {
        $ServerUrl = "http://127.0.0.1:8000"
    }
}

if (Test-Path $credentialsFile) 
{
    Write-Host "[OK] Device credentials found at: $credentialsFile" -ForegroundColor Green

    # Load watch roots from config if it exists
    if (Test-Path $watchConfigFile) 
    {
        try {
            $watchConfig = Get-Content $watchConfigFile -Raw | ConvertFrom-Json
            if ($watchConfig.watch_roots -and (-not $WatchRoots)) {
                $WatchRoots = $watchConfig.watch_roots
                Write-Host "[OK] Watch configuration loaded: $WatchRoots" -ForegroundColor Green
            }
        } catch {
            Write-Host "[WARN] Could not parse watch config file, will prompt for directories." -ForegroundColor Yellow
        }
    }
}
else 
{
    # First-time setup: authentication required
    Write-Host "=== First-Time Setup: User Authentication ===" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "This service requires authentication to register this device."
    Write-Host ""

    # Prompt for credentials
    $username = Read-Host "Enter username"
    $securePassword = Read-Host "Enter password" -AsSecureString
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($securePassword)
    $password = [Runtime.InteropServices.Marshal]::PtrToStringAuto($bstr)
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)

    # Attempt login
    Write-Host ""
    Write-Host "Authenticating with server at: $ServerUrl" -ForegroundColor Yellow
    Write-Host "Sending login request..." -ForegroundColor Yellow

    $loginBody = @{
        username = $username
        password = $password
    } | ConvertTo-Json

    # Clear password from memory
    $password = $null

    try {
        $loginResponse = Invoke-RestMethod -Uri "$ServerUrl/auth/login" `
            -Method POST `
            -ContentType "application/json" `
            -Body $loginBody `
            -TimeoutSec 10
    } catch {
        $errorDetail = ""
        if ($_.ErrorDetails.Message) {
            try {
                $errBody = $_.ErrorDetails.Message | ConvertFrom-Json
                $errorDetail = $errBody.detail
            } catch {
                $errorDetail = $_.ErrorDetails.Message
            }
        }
        
        if ($errorDetail) {
            Write-Host "[X] Authentication failed: $errorDetail" -ForegroundColor Red
        } else {
            Write-Host "[X] Authentication failed" -ForegroundColor Red
            Write-Host "Could not connect to server at: $ServerUrl" -ForegroundColor Red
            Write-Host "Ensure the backend server is running at: $ServerUrl" -ForegroundColor Yellow
            Write-Host ""
            Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Gray
        }
        exit 1
    }

    $jwt = $loginResponse.jwt
    if (-not $jwt) {
        Write-Host "[X] Authentication failed: No JWT token in response" -ForegroundColor Red
        Write-Host "Response: $($loginResponse | ConvertTo-Json -Depth 5)" -ForegroundColor Gray
        exit 1
    }

    Write-Host "[OK] Authentication successful" -ForegroundColor Green

    # Device registration with JWT
    Write-Host "Registering device..." -ForegroundColor Yellow

    $deviceName = "$env:USERNAME@$env:COMPUTERNAME"

    $registerBody = @{
        device_name = $deviceName
        jwt         = $jwt
    } | ConvertTo-Json

    try {
        $registerResponse = Invoke-RestMethod -Uri "$ServerUrl/auth/register-device" `
            -Method POST `
            -ContentType "application/json" `
            -Body $registerBody `
            -TimeoutSec 10
    } catch {
        $errorDetail = ""
        if ($_.ErrorDetails.Message) {
            try {
                $errBody = $_.ErrorDetails.Message | ConvertFrom-Json
                $errorDetail = $errBody.detail
            } catch {
                $errorDetail = $_.ErrorDetails.Message
            }
        }
        Write-Host "[X] Device registration failed" -ForegroundColor Red
        if ($errorDetail) {
            Write-Host "Error: $errorDetail" -ForegroundColor Red
        } else {
            Write-Host "Error: $($_.Exception.Message)" -ForegroundColor Gray
        }
        exit 1
    }

    $deviceId     = $registerResponse.device_id
    $deviceSecret = $registerResponse.device_secret

    if ((-not $deviceId) -or (-not $deviceSecret)) {
        Write-Host "[X] Device registration failed: Missing device_id or device_secret" -ForegroundColor Red
        Write-Host "Response: $($registerResponse | ConvertTo-Json -Depth 5)" -ForegroundColor Gray
        exit 1
    }

    $shortId = $deviceId.Substring(0, [Math]::Min(8, $deviceId.Length))
    Write-Host "[OK] Device registered (ID: $shortId...)" -ForegroundColor Green

    # Save credentials (include username, password, and JWT for auto-login)
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")

    # Re-extract username from loginBody for saving
    $loginParsed = $loginBody | ConvertFrom-Json

    $credentials = @{
        device_id     = $deviceId
        device_secret = $deviceSecret
        username      = $loginParsed.username
        password      = $loginParsed.password
        jwt           = $jwt
        timestamp     = $timestamp
    } | ConvertTo-Json -Depth 5

    # Write without BOM — PowerShell's Set-Content -Encoding UTF8 adds a BOM
    # which breaks JSON parsing in Rust (serde_json)
    [System.IO.File]::WriteAllText($credentialsFile, $credentials, [System.Text.UTF8Encoding]::new($false))
    Write-Host "[OK] Credentials saved to: $credentialsFile" -ForegroundColor Green
}

Write-Host ""

# ============================================================================
# STEP 1: DIRECTORY SELECTION
# ============================================================================

if (-not $WatchRoots) 
{
    Write-Host "=== Directory Selection ===" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Choose which directories to watch:" -ForegroundColor Yellow
    Write-Host "[1] All available drives (C:\, D:\, etc.) - Default"
    Write-Host "[2] User profile directory ($env:USERPROFILE)"
    Write-Host "[3] Custom directories (enter paths separated by semicolon)"
    Write-Host "[4] Skip directory selection (configure manually later)"
    Write-Host ""
    
    $choice = Read-Host "Enter choice (1-4)"
    
    switch ($choice) 
    {
        "1" 
        {
            Write-Host "Selected: All drives" -ForegroundColor Green
            $WatchRoots = ""
        }
        "2" 
        {
            $WatchRoots = $env:USERPROFILE
            Write-Host "Selected: $WatchRoots" -ForegroundColor Green
        }
        "3" 
        {
            Write-Host ""
            Write-Host "Enter directories to watch (separate with semicolon):" -ForegroundColor Yellow
            Write-Host "Example: C:\Users\YourName;D:\Projects;C:\Data" -ForegroundColor Gray
            $WatchRoots = Read-Host "Directories"
            Write-Host "Selected: $WatchRoots" -ForegroundColor Green
        }
        "4" 
        {
            Write-Host "Skipping directory selection. Configure ANYWHERE_DOOR_WATCH_ROOTS manually later." -ForegroundColor Yellow
            $WatchRoots = ""
        }
        default 
        {
            Write-Host "Invalid choice. Using default (all drives)." -ForegroundColor Yellow
            $WatchRoots = ""
        }
    }
}

# Save watch configuration
if ($WatchRoots) 
{
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
    $watchConfigObj = @{
        watch_roots = $WatchRoots
        created_at  = $timestamp
    } | ConvertTo-Json -Depth 5

    [System.IO.File]::WriteAllText($watchConfigFile, $watchConfigObj, [System.Text.UTF8Encoding]::new($false))
    Write-Host "[OK] Watch config saved to: $watchConfigFile" -ForegroundColor Green
}

Write-Host ""

# ============================================================================
# STEP 2: SERVICE INSTALLATION
# ============================================================================

Write-Host "[1/4] Checking for existing service..." -ForegroundColor Cyan

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if ($Recreate -and $existing) 
{
    Write-Host "Removing existing service..." -ForegroundColor Yellow
    sc.exe stop $ServiceName 2>&1 | Out-Null
    Start-Sleep -Seconds 1
    sc.exe delete $ServiceName 2>&1 | Out-Null
    Start-Sleep -Seconds 1
    $existing = $null
    Write-Host "[OK] Existing service removed" -ForegroundColor Green
}

if (-not $existing) 
{
    Write-Host "[OK] No existing service found, will create new one" -ForegroundColor Green
}
else 
{
    Write-Host "[OK] Existing service found, will update configuration" -ForegroundColor Green
}

Write-Host ""
Write-Host "[2/4] Creating/Updating Windows service..." -ForegroundColor Cyan

$binPath = '"' + $resolvedExe + '" --windows-service'

if (-not $existing) 
{
    sc.exe create $ServiceName binPath= $binPath start= auto DisplayName= "$DisplayName" 2>&1 | Out-Null
    sc.exe description $ServiceName "$Description" 2>&1 | Out-Null
}
else 
{
    sc.exe stop $ServiceName 2>&1 | Out-Null
    Start-Sleep -Seconds 1
    sc.exe config $ServiceName binPath= $binPath start= auto DisplayName= "$DisplayName" 2>&1 | Out-Null
    sc.exe description $ServiceName "$Description" 2>&1 | Out-Null
}

Write-Host "[OK] Service created/updated" -ForegroundColor Green

# Configure watch roots via service environment variables
Write-Host ""
Write-Host "[3/4] Configuring watch directories..." -ForegroundColor Cyan

# Services read per-service environment variables from:
# HKLM\SYSTEM\CurrentControlSet\Services\<ServiceName>\Environment (REG_MULTI_SZ)
$regPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$ServiceName"

if (-not (Test-Path $regPath)) 
{
    New-Item -Path $regPath -Force | Out-Null
}

# Use ProgramData for service output so it is independent of service account profile.
$outputDir = "$env:ProgramData\AnywhereDoor"
$metadataOutputPath = "$outputDir\file_event_metadata.ndjson"

# Build service environment list.
$serviceEnv = @(
    "ANYWHERE_DOOR_ENABLE_OS_WATCHER=true",
    "ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT=$metadataOutputPath"
)

if ($WatchRoots) {
    $serviceEnv += "ANYWHERE_DOOR_WATCH_ROOTS=$WatchRoots"
    Write-Host "Watch directories: $WatchRoots" -ForegroundColor Green
} else {
    Write-Host "Watch directories: Auto-detect all drives (default)" -ForegroundColor Green
}

# Also pass credentials path so the service can find it
$serviceEnv += "ANYWHERE_DOOR_CREDENTIALS_PATH=$credentialsFile"

New-ItemProperty -Path $regPath -Name "Environment" -PropertyType MultiString -Value $serviceEnv -Force | Out-Null

# Create output directory
if (-not (Test-Path $outputDir)) 
{
    New-Item -Path $outputDir -ItemType Directory -Force | Out-Null
}

Write-Host "[OK] Watch directories configured" -ForegroundColor Green

Write-Host ""
Write-Host "[4/4] Starting service..." -ForegroundColor Cyan

sc.exe stop $ServiceName 2>&1 | Out-Null
Start-Sleep -Seconds 2
sc.exe start $ServiceName 2>&1 | Out-Null
Start-Sleep -Seconds 2

$status = sc.exe query $ServiceName
Write-Host "[OK] Service started" -ForegroundColor Green

Write-Host ""
Write-Host "=== Installation Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Configuration:" -ForegroundColor Yellow
Write-Host "  Service name:    $ServiceName"
Write-Host "  Display name:    $DisplayName"
Write-Host "  Binary:          $resolvedExe"
Write-Host "  Credentials:     $credentialsFile"
Write-Host "  Watch config:    $watchConfigFile"
Write-Host "  Output directory: $outputDir"
if ($WatchRoots) 
{
    Write-Host "  Watch directories: $WatchRoots"
}
else 
{
    Write-Host "  Watch directories: All available drives (auto-detected)"
}
Write-Host ""
Write-Host "Device Registration:" -ForegroundColor Yellow
Write-Host "  [OK] User authentication completed"
Write-Host "  [OK] Device registered with backend"
Write-Host "  [OK] Credentials securely stored"
Write-Host ""
Write-Host "Useful commands:" -ForegroundColor Yellow
Write-Host "  Get status:     sc.exe query $ServiceName"
Write-Host "  Start service:  sc.exe start $ServiceName"
Write-Host "  Stop service:   sc.exe stop $ServiceName"
Write-Host "  View logs:      Get-Content $metadataOutputPath -Tail 20"
Write-Host ""
Write-Host "To change watch directories later:" -ForegroundColor Cyan
Write-Host "  .\scripts\install-windows-service.ps1 -WatchRoots 'C:\Users;D:\Projects' -Recreate"
Write-Host ""
Write-Host "To reconfigure authentication or directories:" -ForegroundColor Cyan
Write-Host "  1. Remove credentials: Remove-Item $credentialsFile, $watchConfigFile"
Write-Host "  2. Run this installer again"
Write-Host ""
Write-Host "Current service status:" -ForegroundColor Cyan
Write-Host $status
