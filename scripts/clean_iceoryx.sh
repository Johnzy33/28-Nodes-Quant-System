#!/usr/bin/env bash
set -euo pipefail

# scripts/clean_iceoryx.sh
# Safely list and remove Iceoryx artifacts used in /dev/shm and Wine temp folder.
# Usage: ./scripts/clean_iceoryx.sh [-f]

WINE_DIR="$HOME/.wine/dosdevices/c:/Temp/iceoryx2"
SHM_GLOB="/dev/shm/iox2_*"

echo "--- Iceoryx artifact check ---"

# List /dev/shm artifacts and owners
echo "Checking /dev/shm for iox2_* files..."
if compgen -G "$SHM_GLOB" >/dev/null; then
    ls -la $SHM_GLOB || true
    echo
    echo "Owners and details:"
    for f in $SHM_GLOB; do
        stat --printf="%n %U:%G %s bytes\n" "$f" || true
    done
else
    echo "No /dev/shm/iox2_* files found."
fi

# Check Wine temp dir
if [ -d "$WINE_DIR" ]; then
    echo
    echo "Found Wine iceoryx temp directory: $WINE_DIR"
    ls -la "$WINE_DIR" || true
else
    echo
    echo "No Wine iceoryx temp directory at $WINE_DIR"
fi

# Confirm removal unless -f supplied
FORCE=0
if [ "${1:-}" = "-f" ]; then
    FORCE=1
fi

if [ $FORCE -eq 0 ]; then
    echo
    read -p "Remove the above Iceoryx artifacts? [y/N]: " ans
    case "$ans" in
        [yY]|[yY][eE][sS]) ;;
        *) echo "Aborting."; exit 0 ;;
    esac
fi

# Remove /dev/shm files (use sudo if needed)
if compgen -G "$SHM_GLOB" >/dev/null; then
    echo
    echo "Removing /dev/shm/iox2_* files..."
    # Try removing as current user first
    rm -rf /dev/shm/iox2_* 2>/dev/null || true
    # If anything remains, advise using sudo for the remainder
    if compgen -G "$SHM_GLOB" >/dev/null; then
        echo "Some files are owned by another user. Running 'sudo rm -rf /dev/shm/iox2_*' may be required."
        if [ $FORCE -eq 1 ]; then
            sudo rm -rf /dev/shm/iox2_*
        fi
    fi
else
    echo "No /dev/shm artifacts to remove."
fi

# Remove Wine temp dir contents
if [ -d "$WINE_DIR" ]; then
    echo
    echo "Removing contents of $WINE_DIR ..."
    rm -rf "$WINE_DIR"/* || true
    echo "Done removing Wine artifacts."
fi

echo
echo "Cleanup complete."
exit 0
