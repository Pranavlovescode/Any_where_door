@echo off
REM AnywhereDoor Server startup script for Windows
REM Usage: run_server.bat [options]
REM   --production : Run in production mode (no reload)
REM   --port PORT  : Specify port (default: 8000)

setlocal enabledelayedexpansion

REM Defaults
set PRODUCTION=false
set PORT=8000
set HOST=0.0.0.0
set RELOAD=--reload

REM Parse arguments
:parse_args
if "%~1"=="" goto args_done
if "%~1"=="--production" (
    set PRODUCTION=true
    set RELOAD=
    shift
    goto parse_args
)
if "%~1"=="--port" (
    set PORT=%~2
    shift
    shift
    goto parse_args
)
shift
goto parse_args

:args_done
cls
echo.
echo ============================================
echo   AnywhereDoor Server
echo ============================================
echo.

REM Check if Python is installed
python --version >nul 2>&1
if errorlevel 1 (
    echo [X] Python is not installed or not in PATH
    pause
    exit /b 1
)

echo [OK] Python found

REM Check if venv exists, create if not
if not exist "venv" (
    echo [INFO] Creating Python virtual environment...
    python -m venv venv
    echo [OK] Virtual environment created
)

REM Activate virtual environment
echo [INFO] Activating virtual environment...
call venv\Scripts\activate.bat
echo [OK] Virtual environment activated

REM Install requirements
echo [INFO] Checking dependencies...
if exist "requirements.txt" (
    pip install -q -r requirements.txt >nul 2>&1
    if errorlevel 1 (
        echo [X] Failed to install dependencies
        pause
        exit /b 1
    )
    echo [OK] Dependencies installed
) else (
    echo [X] requirements.txt not found
    pause
    exit /b 1
)

REM Create .env if it doesn't exist
if not exist ".env" (
    echo [INFO] Creating .env file from template...
    if exist ".env.example" (
        copy .env.example .env >nul
        echo [OK] .env created
        echo [WARN] Remember to update JWT_SECRET in .env for production
    )
)

REM Create storage directory
if not exist "storage\files" (
    mkdir storage\files
)
echo [OK] Storage directory ready

echo.
echo ============================================
echo   Starting Server
echo ============================================
echo.

if "%PRODUCTION%"=="true" (
    echo [INFO] Running in PRODUCTION mode
    uvicorn main:app --host %HOST% --port %PORT% --workers 4 --log-level info
) else (
    echo [INFO] Running in DEVELOPMENT mode (with auto-reload)
    echo [INFO] Server running at: http://%HOST%:%PORT%
    echo [INFO] API docs at: http://%HOST%:%PORT%/docs
    echo [INFO] Press Ctrl+C to stop
    echo.
    uvicorn main:app %RELOAD% --host %HOST% --port %PORT% --log-level info
)

pause
