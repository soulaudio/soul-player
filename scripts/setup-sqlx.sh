#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Soul Player SQLx Setup ===${NC}\n"

# Check if .env exists, if not copy from .env.example
if [ ! -f .env ]; then
    echo -e "${YELLOW}Creating .env from .env.example...${NC}"
    cp .env.example .env
    echo -e "${GREEN}✓ Created .env${NC}\n"
else
    echo -e "${GREEN}✓ .env already exists${NC}\n"
fi

# Load environment variables from .env
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | grep -v '^$' | xargs)
fi

# Validate DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo -e "${RED}ERROR: DATABASE_URL not set in .env${NC}"
    echo "Please ensure .env contains: DATABASE_URL=sqlite:libraries/soul-storage/.tmp/dev.db"
    exit 1
fi

echo -e "${BLUE}Using DATABASE_URL: ${NC}$DATABASE_URL\n"

# Create .tmp directory if it doesn't exist
TMP_DIR="libraries/soul-storage/.tmp"
if [ ! -d "$TMP_DIR" ]; then
    echo -e "${YELLOW}Creating directory: $TMP_DIR${NC}"
    mkdir -p "$TMP_DIR"
    echo -e "${GREEN}✓ Created $TMP_DIR${NC}\n"
else
    echo -e "${GREEN}✓ Directory $TMP_DIR exists${NC}\n"
fi

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ ERROR: cargo not found in bash PATH${NC}"
    echo ""
    echo -e "${YELLOW}On Windows, run these commands directly in PowerShell instead:${NC}"
    echo ""
    echo -e "  ${BLUE}# Install sqlx-cli (if not installed)${NC}"
    echo -e "  cargo install sqlx-cli --no-default-features --features sqlite"
    echo ""
    echo -e "  ${BLUE}# Create database and run migrations${NC}"
    echo -e "  sqlx database create"
    echo -e "  sqlx migrate run --source libraries/soul-storage/migrations"
    echo ""
    echo -e "  ${BLUE}# Prepare offline mode (optional)${NC}"
    echo -e "  cd libraries/soul-storage"
    echo -e "  cargo sqlx prepare -- --lib"
    echo -e "  cd ../.."
    echo ""
    echo -e "  ${BLUE}# Verify${NC}"
    echo -e "  cargo check -p soul-storage"
    echo ""
    exit 1
fi

# Check if sqlx-cli is installed
if ! command -v sqlx &> /dev/null; then
    echo -e "${YELLOW}sqlx-cli not found. Installing...${NC}"
    cargo install sqlx-cli --no-default-features --features sqlite
    echo -e "${GREEN}✓ Installed sqlx-cli${NC}\n"
else
    echo -e "${GREEN}✓ sqlx-cli is installed${NC}\n"
fi

# Create database if it doesn't exist
echo -e "${BLUE}Setting up database...${NC}"
sqlx database create
echo -e "${GREEN}✓ Database ready${NC}\n"

# Run migrations
echo -e "${BLUE}Running migrations...${NC}"
sqlx migrate run --source libraries/soul-storage/migrations
echo -e "${GREEN}✓ Migrations applied${NC}\n"

# Prepare SQLx offline mode (optional but recommended)
echo -e "${BLUE}Preparing SQLx offline mode...${NC}"
# Use absolute path for DATABASE_URL when running prepare
ABS_DB_PATH="$(pwd)/libraries/soul-storage/.tmp/dev.db"
if (cd libraries/soul-storage && DATABASE_URL="sqlite://$ABS_DB_PATH" cargo sqlx prepare -- --lib 2>/dev/null); then
    echo -e "${GREEN}✓ SQLx offline data prepared${NC}\n"
else
    echo -e "${YELLOW}⚠ SQLx offline mode preparation skipped (compilation errors)${NC}"
    echo -e "${YELLOW}  This is optional - SQLx will work fine using the database${NC}\n"
fi

# Verify everything works
echo -e "${BLUE}Verifying setup...${NC}"
if cargo check -p soul-storage 2>&1 | grep -q "error"; then
    echo -e "${RED}✗ Verification failed. Check errors above.${NC}"
    exit 1
else
    echo -e "${GREEN}✓ Verification successful!${NC}\n"
fi

echo -e "${GREEN}=== Setup Complete! ===${NC}\n"
echo -e "You can now:"
echo -e "  ${BLUE}•${NC} Run ${YELLOW}cargo build${NC} - SQLx will verify queries at compile time"
echo -e "  ${BLUE}•${NC} Run ${YELLOW}cargo test${NC} - Tests will use testcontainers"
echo -e "  ${BLUE}•${NC} Use ${YELLOW}SQLX_OFFLINE=true${NC} for CI/offline builds"
echo -e ""
echo -e "Database location: ${YELLOW}$DATABASE_URL${NC}"
echo -e ""
