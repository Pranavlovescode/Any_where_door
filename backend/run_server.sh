#!/bin/bash

# AnywhereDoor Server startup script
# Usage: ./run_server.sh [options]
#   --production : Run in production mode (no reload)
#   --port PORT  : Specify port (default: 8000)
#   --host HOST  : Specify host (default: 0.0.0.0)

# Color codes for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Defaults
PRODUCTION=false
PORT=8000
HOST="0.0.0.0"
RELOAD="--reload"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --production)
            PRODUCTION=true
            RELOAD=""
            shift
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --host)
            HOST="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}╔════════════════════════════════════════════${NC}"
echo -e "${BLUE}║${NC} ${GREEN}AnywhereDoor Server${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════${NC}"
echo ""

# Check if Python is installed
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}✗ Python 3 is not installed${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Python 3 found${NC}"

# Check if venv exists, create if not
if [ ! -d "venv" ]; then
    echo -e "${YELLOW}ℹ Creating Python virtual environment...${NC}"
    python3 -m venv venv
    echo -e "${GREEN}✓ Virtual environment created${NC}"
fi

# Activate virtual environment
echo -e "${YELLOW}ℹ Activating virtual environment...${NC}"
source venv/bin/activate
echo -e "${GREEN}✓ Virtual environment activated${NC}"

# Install requirements
echo -e "${YELLOW}ℹ Checking dependencies...${NC}"
if [ -f "requirements.txt" ]; then
    pip install -q -r requirements.txt 2>/dev/null
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Dependencies installed${NC}"
    else
        echo -e "${RED}✗ Failed to install dependencies${NC}"
        exit 1
    fi
else
    echo -e "${RED}✗ requirements.txt not found${NC}"
    exit 1
fi

# Create .env if it doesn't exist
if [ ! -f ".env" ]; then
    echo -e "${YELLOW}ℹ Creating .env file from template...${NC}"
    if [ -f ".env.example" ]; then
        cp .env.example .env
        echo -e "${GREEN}✓ .env created${NC}"
        echo -e "${YELLOW}⚠ Remember to update JWT_SECRET in .env for production${NC}"
    fi
fi

# Create storage directory
mkdir -p storage/files
echo -e "${GREEN}✓ Storage directory ready${NC}"

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════${NC}"
echo -e "${BLUE}║${NC} Starting Server"
echo -e "${BLUE}╚════════════════════════════════════════════${NC}"
echo ""

if [ "$PRODUCTION" = true ]; then
    echo -e "${YELLOW}ℹ Running in PRODUCTION mode${NC}"
    uvicorn main:app --host "$HOST" --port "$PORT" --workers 4 --log-level info
else
    echo -e "${YELLOW}ℹ Running in DEVELOPMENT mode (with auto-reload)${NC}"
    echo -e "${GREEN}📍 Server running at: http://$HOST:$PORT${NC}"
    echo -e "${GREEN}📖 API docs at: http://$HOST:$PORT/docs${NC}"
    echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
    echo ""
    uvicorn main:app $RELOAD --host "$HOST" --port "$PORT" --log-level info
fi
