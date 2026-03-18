param(
  [string]$ServiceName = "AnywhereDoorAgent",
  [string]$DisplayName = "Anywhere Door Agent",
  [string]$Description = "Anywhere Door background agent (Rust)",
  [string]$ExePath = "$PSScriptRoot\..\target\release\anywhere_door_agent.exe",
  [switch]$Recreate
)

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
  throw "Run this script in an elevated PowerShell session (Run as Administrator)."
}

if (-not (Test-Path $ExePath)) {
  throw "Rust binary not found at: $ExePath`nBuild first with: cargo build --release"
}

$resolvedExe = (Resolve-Path $ExePath).Path
$binPath = '"' + $resolvedExe + '" --windows-service'

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($Recreate -and $existing) {
  sc.exe stop $ServiceName | Out-Null
  sc.exe delete $ServiceName | Out-Null
  Start-Sleep -Seconds 1
  $existing = $null
}

if (-not $existing) {
  sc.exe create $ServiceName binPath= $binPath start= auto DisplayName= "$DisplayName"
  sc.exe description $ServiceName "$Description"
} else {
  sc.exe stop $ServiceName | Out-Null
  sc.exe config $ServiceName binPath= $binPath start= auto DisplayName= "$DisplayName"
  sc.exe description $ServiceName "$Description"
}

sc.exe start $ServiceName
sc.exe query $ServiceName
