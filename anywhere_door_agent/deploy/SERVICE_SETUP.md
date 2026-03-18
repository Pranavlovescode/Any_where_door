# Anywhere Door Agent Service Setup

This project now supports:
- Linux `systemd` service management
- Windows Service Control Manager (SCM) registration

## Linux (systemd)

1. Build release binary:

   cargo build --release

2. Install and start service:

   ./scripts/install-systemd.sh

3. Useful commands:

   sudo systemctl status anywhere-door-agent.service
   sudo systemctl restart anywhere-door-agent.service
   sudo systemctl stop anywhere-door-agent.service
   sudo systemctl disable anywhere-door-agent.service

The unit file is:
- deploy/linux/anywhere-door-agent.service

## Windows (SCM)

1. Build release binary:

   cargo build --release

2. Open PowerShell as Administrator.

3. Register and start the service:

   .\scripts\register-windows-service.ps1

4. Useful commands:

   sc.exe query AnywhereDoorAgent
   sc.exe stop AnywhereDoorAgent
   sc.exe start AnywhereDoorAgent

The service runs the Rust binary in service mode using:
- --windows-service
