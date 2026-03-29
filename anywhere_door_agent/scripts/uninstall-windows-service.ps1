param(
    [string]$ServiceName = "AnywhereDoorAgent"
)

# Require administrator privileges
$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    throw "ERROR: Run this script in an elevated PowerShell session (Run as Administrator)."
}

Write-Host "=== Uninstalling Anywhere Door Agent ===" -ForegroundColor Red
Write-Host ""

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if (-not $existing) {
    Write-Host "Service '$ServiceName' not found. Nothing to uninstall." -ForegroundColor Yellow
    exit 0
}

Write-Host "[1/3] Stopping service..." -ForegroundColor Cyan
sc.exe stop $ServiceName 2>&1 | Out-Null
Start-Sleep -Seconds 1
Write-Host "[OK] Service stopped" -ForegroundColor Green

Write-Host "[2/3] Removing service..." -ForegroundColor Cyan
sc.exe delete $ServiceName 2>&1 | Out-Null
Start-Sleep -Seconds 1
Write-Host "[OK] Service removed" -ForegroundColor Green

# Optional: Remove configuration from registry
$regPath = "HKLM:\SYSTEM\CurrentControlSet\Services\$ServiceName"
if (Test-Path $regPath) {
    Write-Host "[3/3] Cleaning up registry..." -ForegroundColor Cyan
    Remove-Item -Path $regPath -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "[OK] Registry cleaned" -ForegroundColor Green
} else {
    Write-Host "[3/3] Registry cleanup..." -ForegroundColor Cyan
    Write-Host "[OK] No registry entries found" -ForegroundColor Green
}

Write-Host ""
Write-Host "=== Uninstall Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Removed:" -ForegroundColor Yellow
Write-Host "  [OK] Service: $ServiceName"
Write-Host "  [OK] Registry entries"
Write-Host ""
Write-Host "Note: Data files were not deleted." -ForegroundColor Gray
Write-Host "Possible locations:" -ForegroundColor Gray
Write-Host "  $env:ProgramData\AnywhereDoor" -ForegroundColor Gray
Write-Host "  $env:APPDATA\AnywhereDoor" -ForegroundColor Gray
Write-Host "To remove manually:" -ForegroundColor Gray
Write-Host "  Remove-Item $env:ProgramData\AnywhereDoor -Recurse -Force" -ForegroundColor Gray
Write-Host "  Remove-Item $env:APPDATA\AnywhereDoor -Recurse -Force" -ForegroundColor Gray
