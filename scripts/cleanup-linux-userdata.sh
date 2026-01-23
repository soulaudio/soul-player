#!/bin/bash
# Soul Player Linux User Data Cleanup Script
#
# This script removes all Soul Player user data from your system.
# Run this AFTER uninstalling Soul Player to completely reset the app state.
#
# WARNING: This will delete:
#   - Database (all your music library, playlists, settings)
#   - Logs
#   - Configuration files
#   - Cached artwork
#
# Usage:
#   ./cleanup-linux-userdata.sh         # Interactive mode (asks for confirmation)
#   ./cleanup-linux-userdata.sh --force # Skip confirmation (for scripts)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Determine data directory based on XDG_CONFIG_HOME or default
if [ -n "$XDG_CONFIG_HOME" ]; then
    DATA_DIR="$XDG_CONFIG_HOME/soul-player"
else
    DATA_DIR="$HOME/.config/soul-player"
fi

# Check if data directory exists
if [ ! -d "$DATA_DIR" ]; then
    echo -e "${YELLOW}No Soul Player user data found at: $DATA_DIR${NC}"
    echo "Nothing to clean up."
    exit 0
fi

# Show what will be deleted
echo -e "${YELLOW}=== Soul Player User Data Cleanup ===${NC}"
echo ""
echo "The following directory will be PERMANENTLY DELETED:"
echo -e "${RED}$DATA_DIR${NC}"
echo ""
echo "This includes:"
echo "  - Database (soul-player.db) with all tracks, playlists, and settings"
echo "  - Logs directory"
echo "  - Configuration files (config.json)"
echo "  - Library files (managed music files)"
echo ""

# Calculate size
SIZE=$(du -sh "$DATA_DIR" 2>/dev/null | cut -f1)
echo "Total size: $SIZE"
echo ""

# Ask for confirmation unless --force is passed
if [ "$1" != "--force" ]; then
    echo -e "${YELLOW}WARNING: This action cannot be undone!${NC}"
    read -p "Are you sure you want to delete all Soul Player data? (yes/no): " CONFIRM
    if [ "$CONFIRM" != "yes" ]; then
        echo "Cleanup cancelled."
        exit 0
    fi
fi

# Perform cleanup
echo ""
echo "Removing Soul Player user data..."
rm -rf "$DATA_DIR"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Successfully removed all Soul Player user data${NC}"
    echo ""
    echo "You can now:"
    echo "  - Reinstall Soul Player for a fresh start"
    echo "  - Or keep it uninstalled"
else
    echo -e "${RED}✗ Failed to remove user data${NC}"
    echo "You may need to run this script with sudo or check permissions."
    exit 1
fi
