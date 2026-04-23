# Anywhere Door — Installation Guide

> Cross-platform installation instructions for the **Anywhere Door** system.
> This covers the **Backend Server** (Python / FastAPI) and the **Agent** (Rust).

---

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Windows Installation](#windows-installation)
  - [Prerequisites (Windows)](#prerequisites-windows)
  - [Step 1 — Clone the Repository](#step-1--clone-the-repository)
  - [Step 2 — Start the Backend Server](#step-2--start-the-backend-server-windows)
  - [Step 3 — Build the Agent](#step-3--build-the-agent-windows)
  - [Step 4 — Run the Agent (First-Time Setup)](#step-4--run-the-agent-first-time-setup-windows)
  - [Step 5 — Install as a Windows Service](#step-5--install-as-a-windows-service)
  - [Step 6 — Verify Installation](#step-6--verify-installation-windows)
- [Linux Installation](#linux-installation)
  - [Prerequisites (Linux)](#prerequisites-linux)
  - [Step 1 — Clone the Repository](#step-1--clone-the-repository-1)
  - [Step 2 — Start the Backend Server](#step-2--start-the-backend-server-linux)
  - [Step 3 — Build the Agent](#step-3--build-the-agent-linux)
  - [Step 4 — Run the Agent (First-Time Setup)](#step-4--run-the-agent-first-time-setup-linux)
  - [Step 5 — Install as a systemd Service](#step-5--install-as-a-systemd-service)
  - [Step 6 — Verify Installation](#step-6--verify-installation-linux)
- [Configuration Reference](#configuration-reference)
- [Managing the Service](#managing-the-service)
- [Changing Watch Directories](#changing-watch-directories)
- [Uninstallation](#uninstallation)
- [Troubleshooting](#troubleshooting)

---

## Architecture Overview

```
┌──────────────────────┐          HTTP / REST          ┌──────────────────────┐
│   Anywhere Door      │  ◄──────────────────────────► │   Backend Server     │
│   Agent (Rust)       │   device auth, file events    │   (Python / FastAPI) │
│                      │                               │                      │
│  • Watches files     │                               │  • User auth (JWT)   │
│  • Detects changes   │                               │  • Device registry   │
│  • Streams events    │                               │  • File sync API     │
│  • Runs as service   │                               │  • SQLite / Postgres │
└──────────────────────┘                               └──────────────────────┘
```

The system has two components:

| Component         | Language | Purpose                                        |
| ----------------- | -------- | ---------------------------------------------- |
| **Backend Server** | Python   | REST API for auth, device registration, sync   |
| **Agent**          | Rust     | OS-level file watcher, runs as a background service |

You must set up the **backend server first**, then build and install the **agent**.

---

# Windows Installation

## Prerequisites (Windows)

Install the following before proceeding:

### 1. Git

Download and install from [https://git-scm.com/download/win](https://git-scm.com/download/win).

Verify:
```powershell
git --version
```

### 2. Python 3.10+

Download from [https://www.python.org/downloads/](https://www.python.org/downloads/).

> **Important:** Check **"Add Python to PATH"** during installation.

Verify:
```powershell
python --version
```

### 3. Rust Toolchain

Install via `rustup`:
```powershell
# Download and run the installer from https://rustup.rs
# Or use winget:
winget install Rustlang.Rustup
```

After installation, restart your terminal and verify:
```powershell
rustc --version
cargo --version
```

### 4. Visual Studio Build Tools (C++ workload)

Rust on Windows requires the MSVC C++ build tools.

- Download from: [https://visualstudio.microsoft.com/visual-cpp-build-tools/](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- During installation, select **"Desktop development with C++"**.

> If you already have Visual Studio installed with the C++ workload, you can skip this.

---

## Step 1 — Clone the Repository

```powershell
git clone https://github.com/Pranavlovescode/Any_where_door.git
cd Any_where_door
```

---

## Step 2 — Start the Backend Server (Windows)

The backend must be running before the agent can authenticate and register.

### Option A: Use the provided script (recommended)

```powershell
cd backend
.\run_server.bat
```

This script will automatically:
- Check for Python
- Create a virtual environment (`venv`)
- Install dependencies from `requirements.txt`
- Create `.env` from `.env.example` if it doesn't exist
- Create the `storage/files` directory
- Start the Uvicorn server on `http://0.0.0.0:8000`

### Option B: Manual setup

```powershell
cd backend

# Create and activate virtual environment
python -m venv venv
.\venv\Scripts\Activate.ps1

# Install dependencies
pip install -r requirements.txt

# Create .env file
copy .env.example .env

# (Optional) Edit .env to change JWT_SECRET, database URL, etc.
notepad .env

# Create storage directory
mkdir storage\files

# Start the server
uvicorn main:app --reload --host 0.0.0.0 --port 8000
```

### Verify the server is running

Open your browser and visit:
- **API root:** [http://localhost:8000](http://localhost:8000)
- **Interactive API docs:** [http://localhost:8000/docs](http://localhost:8000/docs)

> **Note:** Keep this terminal open. The backend must stay running while you set up the agent.

---

## Step 3 — Build the Agent (Windows)

Open a **new terminal** and navigate to the agent directory:

```powershell
cd anywhere_door_agent

# Build the release binary
cargo build --release
```

This will produce:
```
target\release\anywhere_door_agent.exe
```

> First build may take a few minutes while Cargo downloads and compiles dependencies.

---

## Step 4 — Run the Agent (First-Time Setup, Windows)

Before installing as a service, run the agent interactively to complete the first-time setup:

```powershell
.\target\release\anywhere_door_agent.exe
```

The agent will walk you through:

1. **User Authentication** — Enter your username and password (the same credentials used for the backend).
2. **Device Registration** — The agent registers this machine with the backend and receives a device ID and secret.
3. **Directory Selection** — Choose which directories to monitor:
   - `[1]` Home directory
   - `[2]` Custom directories
   - `[3]` Specific application data directory
   - `[4]` Skip (configure later)

Credentials are saved to:
- `%USERPROFILE%\.anywheredoor` (device ID + secret)
- `%USERPROFILE%\.anywheredoor_watch_roots` (watch configuration)

Press `Ctrl+C` to stop the agent after setup is confirmed.

---

## Step 5 — Install as a Windows Service

Open **PowerShell as Administrator** (right-click → "Run as Administrator"):

```powershell
cd anywhere_door_agent

# Run the service installer
.\scripts\install-windows-service.ps1
```

The installer will:
1. Verify the compiled binary exists
2. Prompt you to select watch directories (all drives, user profile, custom, or skip)
3. Create/update the Windows Service (`AnywhereDoorAgent`)
4. Configure environment variables in the registry
5. Create the output directory (`%ProgramData%\AnywhereDoor`)
6. Start the service

### Advanced installer options

```powershell
# Specify watch directories directly (no prompt)
.\scripts\install-windows-service.ps1 -WatchRoots 'C:\Users;D:\Projects'

# Force a clean reinstall
.\scripts\install-windows-service.ps1 -Recreate

# Custom service name
.\scripts\install-windows-service.ps1 -ServiceName "MyAgentService"
```

---

## Step 6 — Verify Installation (Windows)

```powershell
# Check service status
sc.exe query AnywhereDoorAgent

# View live event logs
Get-Content $env:ProgramData\AnywhereDoor\file_event_metadata.ndjson -Tail 20

# Watch logs in real-time
Get-Content $env:ProgramData\AnywhereDoor\file_event_metadata.ndjson -Wait
```

The service starts automatically on boot. You can also manage it from the **Services** GUI (`services.msc`).

---

# Linux Installation

## Prerequisites (Linux)

### Ubuntu / Debian

```bash
# System packages
sudo apt update
sudo apt install -y git curl build-essential python3 python3-pip python3-venv

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Fedora / RHEL / CentOS

```bash
# System packages
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y git curl python3 python3-pip

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Arch Linux

```bash
sudo pacman -S --needed git curl base-devel python python-pip rust
```

### Verify prerequisites

```bash
git --version
python3 --version
rustc --version
cargo --version
```

---

## Step 1 — Clone the Repository

```bash
git clone https://github.com/Pranavlovescode/Any_where_door.git
cd Any_where_door
```

---

## Step 2 — Start the Backend Server (Linux)

### Option A: Use the provided script (recommended)

```bash
cd backend
chmod +x run_server.sh
./run_server.sh
```

This script will automatically:
- Check for Python 3
- Create a virtual environment (`venv`)
- Install dependencies from `requirements.txt`
- Create `.env` from `.env.example` if needed
- Create the `storage/files` directory
- Start the Uvicorn server on `http://0.0.0.0:8000`

### Option B: Manual setup

```bash
cd backend

# Create and activate virtual environment
python3 -m venv venv
source venv/bin/activate

# Install dependencies
pip install -r requirements.txt

# Create .env file
cp .env.example .env

# (Optional) Edit configuration
nano .env

# Create storage directory
mkdir -p storage/files

# Start the server
uvicorn main:app --reload --host 0.0.0.0 --port 8000
```

### Verify the server is running

```bash
curl http://localhost:8000/docs
# Or open http://localhost:8000/docs in a browser
```

> **Note:** Keep this terminal open. The backend must stay running while you set up the agent.

---

## Step 3 — Build the Agent (Linux)

Open a **new terminal**:

```bash
cd anywhere_door_agent

# Build the release binary
cargo build --release
```

This will produce:
```
target/release/anywhere_door_agent
```

---

## Step 4 — Run the Agent (First-Time Setup, Linux)

Run the agent interactively to complete the first-time setup:

```bash
./target/release/anywhere_door_agent
```

The agent will guide you through:

1. **User Authentication** — Enter your username and password.
2. **Device Registration** — Registers this machine with the backend.
3. **Directory Selection** — Choose directories to watch:
   - `[1]` Home directory
   - `[2]` Custom directories (comma-separated)
   - `[3]` Specific application data directories
   - `[4]` Skip (configure later)

Credentials are saved to:
- `~/.anywheredoor` (device ID + secret)
- `~/.anywheredoor_watch_roots` (watch configuration)

Press `Ctrl+C` to stop the agent after setup completes.

---

## Step 5 — Install as a systemd Service

The provided installer handles everything — binary installation, systemd unit creation, user authentication, and directory selection:

```bash
cd anywhere_door_agent
chmod +x scripts/install-systemd.sh
sudo ./scripts/install-systemd.sh
```

The script will:
1. Authenticate you with the backend (if first time)
2. Register the device
3. Prompt you to select watch directories:
   - `[1]` Home directory (default)
   - `[2]` Entire filesystem (`/`)
   - `[3]` Custom directories (comma-separated)
   - `[4]` Skip and configure manually later
4. Install the binary to `/usr/local/bin/anywhere_door_agent`
5. Create log directory at `/var/log/anywhere-door-agent/`
6. Create and enable the systemd service
7. Start the service immediately

### Skip interactive prompts (headless install)

If credentials and watch roots are already configured:

```bash
sudo ANYWHERE_DOOR_WATCH_ROOTS=/home/myuser,/var/data ./scripts/install-systemd.sh
```

---

## Step 6 — Verify Installation (Linux)

```bash
# Check service status
sudo systemctl status anywhere-door-agent.service

# View live event logs
tail -f /var/log/anywhere-door-agent/file_event_metadata.ndjson

# Check journal logs
journalctl -u anywhere-door-agent.service -f
```

The service starts automatically on boot.

---

# Configuration Reference

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ANYWHERE_DOOR_SERVER_URL` | Backend server URL | `http://127.0.0.1:8000` |
| `ANYWHERE_DOOR_ENABLE_OS_WATCHER` | Enable/disable file watcher | `true` |
| `ANYWHERE_DOOR_WATCH_ROOTS` | Directories to watch | *(none — interactive prompt)* |
| `ANYWHERE_DOOR_FILE_EVENT_METADATA_OUTPUT` | Path to event log file | `output/file_event_metadata.ndjson` |
| `ANYWHERE_DOOR_CREDENTIALS_PATH` | Path to device credentials file | `~/.anywheredoor` |
| `ANYWHERE_DOOR_CONFIG_PATH` | Path to watch roots config file | `~/.anywheredoor_watch_roots` |

## Backend `.env` Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | Database connection string | `sqlite:///./any_where_door.db` |
| `JWT_SECRET` | Secret key for JWT tokens | `your-super-secret-key-...` |
| `JWT_ALGORITHM` | JWT signing algorithm | `HS256` |
| `JWT_EXPIRATION_HOURS` | JWT token expiration | `24` |
| `FILE_STORAGE_DIR` | File storage directory | `./storage/files` |
| `MAX_FILE_SIZE_MB` | Max upload file size | `500` |
| `HOST` | Server bind address | `0.0.0.0` |
| `PORT` | Server port | `8000` |

## Watch Roots Path Separator

| OS | Separator | Example |
|----|-----------|---------|
| **Linux** | Comma (`,`) | `/home/user,/var/log,/opt/data` |
| **Windows** | Semicolon (`;`) | `C:\Users;D:\Projects;C:\Data` |

---

# Managing the Service

## Windows

```powershell
# Check status
sc.exe query AnywhereDoorAgent

# Start
sc.exe start AnywhereDoorAgent

# Stop
sc.exe stop AnywhereDoorAgent

# Restart (stop + start)
sc.exe stop AnywhereDoorAgent; Start-Sleep 2; sc.exe start AnywhereDoorAgent

# View in Services GUI
services.msc
```

## Linux

```bash
# Check status
sudo systemctl status anywhere-door-agent.service

# Start
sudo systemctl start anywhere-door-agent.service

# Stop
sudo systemctl stop anywhere-door-agent.service

# Restart
sudo systemctl restart anywhere-door-agent.service

# Disable auto-start on boot
sudo systemctl disable anywhere-door-agent.service

# Enable auto-start on boot
sudo systemctl enable anywhere-door-agent.service
```

---

# Changing Watch Directories

## Windows

**Option 1:** Reinstall with new directories:
```powershell
.\scripts\install-windows-service.ps1 -WatchRoots 'C:\Users;D:\Projects' -Recreate
```

**Option 2:** Edit registry manually:
```powershell
$regPath = 'HKLM:\SYSTEM\CurrentControlSet\Services\AnywhereDoorAgent'
# View current config
Get-ItemProperty -Path $regPath -Name Environment

# Update watch roots in the Environment multi-string
# Then restart the service
sc.exe stop AnywhereDoorAgent
sc.exe start AnywhereDoorAgent
```

## Linux

**Option 1:** Edit the systemd unit file:
```bash
sudo systemctl edit --full anywhere-door-agent.service
# Modify the ANYWHERE_DOOR_WATCH_ROOTS line
sudo systemctl daemon-reload
sudo systemctl restart anywhere-door-agent.service
```

**Option 2:** Re-run the installer:
```bash
sudo ANYWHERE_DOOR_WATCH_ROOTS=/new/path1,/new/path2 ./scripts/install-systemd.sh
```

---

# Uninstallation

## Windows

Open **PowerShell as Administrator**:

```powershell
cd anywhere_door_agent
.\scripts\uninstall-windows-service.ps1
```

This will:
- Stop the running service
- Remove the service from Windows
- Clean up registry entries

Data files are **not deleted** automatically. To remove them:
```powershell
Remove-Item "$env:ProgramData\AnywhereDoor" -Recurse -Force
Remove-Item "$env:APPDATA\AnywhereDoor" -Recurse -Force
```

## Linux

```bash
cd anywhere_door_agent
sudo ./scripts/uninstall.sh
```

This will:
- Stop and disable the systemd service
- Remove the systemd unit file
- Remove the binary from `/usr/local/bin/`
- Remove the service user (if created)
- Remove log and working directories

To also remove credential files:
```bash
rm ~/.anywheredoor ~/.anywheredoor_watch_roots
```

---

# Troubleshooting

## General

| Problem | Solution |
|---------|----------|
| Agent can't connect to backend | Ensure the backend server is running on `http://127.0.0.1:8000`. Check `ANYWHERE_DOOR_SERVER_URL`. |
| Authentication fails | Verify your username/password. Try logging in via the API docs at `/docs`. |
| "No watch directories configured" | Set `ANYWHERE_DOOR_WATCH_ROOTS` or re-run the agent interactively to configure. |
| Agent builds but won't start | Check that all env vars are set. Run the binary manually to see error output. |

## Windows-Specific

| Problem | Solution |
|---------|----------|
| `cargo build` fails with linker errors | Install Visual Studio Build Tools with C++ workload. |
| "Run this script in an elevated PowerShell session" | Right-click PowerShell → **Run as Administrator**. |
| Service won't start | Check **Event Viewer → Windows Logs → System** for errors. Try `.\scripts\install-windows-service.ps1 -Recreate`. |
| "Rust binary not found" | Run `cargo build --release` first. Binary should be at `target\release\anywhere_door_agent.exe`. |
| PowerShell script execution disabled | Run `Set-ExecutionPolicy RemoteSigned -Scope CurrentUser` to allow scripts. |

## Linux-Specific

| Problem | Solution |
|---------|----------|
| `cargo` not found after install | Run `source "$HOME/.cargo/env"` or restart your terminal. |
| "systemctl not found" | Your system doesn't use systemd. Use a manual init script instead. |
| Permission denied on watched directory | Ensure the service user has read access: `ls -ld /path/to/dir`. |
| High CPU usage | Reduce the scope of watched directories. Avoid watching `/` unless necessary. |
| "Missing release binary" | Run `cargo build --release` from the `anywhere_door_agent` directory. |

## Output Format

Both platforms output filesystem events in **NDJSON** (Newline-Delimited JSON) format:

```json
{"timestamp_epoch_ms":1234567890,"event_kind":"create","paths":[{"path":"/home/user/file.txt","exists":true,"is_dir":false,"size_bytes":1024,"modified_epoch_ms":1234567890}]}
{"timestamp_epoch_ms":1234567891,"event_kind":"modify","paths":[{"path":"/home/user/file.txt","exists":true,"is_dir":false,"size_bytes":2048,"modified_epoch_ms":1234567891}]}
```

Event kinds: `create`, `modify`, `remove`, `rename`, `access`, `other`.

---

## Quick Reference Card

### Windows (PowerShell)

```powershell
# Full setup from scratch
git clone https://github.com/Pranavlovescode/Any_where_door.git
cd Any_where_door\backend
.\run_server.bat                                          # Terminal 1: start backend
# --- new terminal ---
cd Any_where_door\anywhere_door_agent
cargo build --release                                     # build agent
.\target\release\anywhere_door_agent.exe                  # first-time setup
# --- elevated PowerShell ---
.\scripts\install-windows-service.ps1                     # install as service
sc.exe query AnywhereDoorAgent                            # verify
```

### Linux (Bash)

```bash
# Full setup from scratch
git clone https://github.com/Pranavlovescode/Any_where_door.git
cd Any_where_door/backend
./run_server.sh                                           # Terminal 1: start backend
# --- new terminal ---
cd Any_where_door/anywhere_door_agent
cargo build --release                                     # build agent
./target/release/anywhere_door_agent                      # first-time setup
sudo ./scripts/install-systemd.sh                         # install as service
sudo systemctl status anywhere-door-agent.service         # verify
```
