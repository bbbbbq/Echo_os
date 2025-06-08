#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

IMAGE_NAME="fs.img"
IMAGE_SIZE_MB=32
MOUNT_POINT="/mnt/echo_os_tmp_fs"

echo "=== Preparing to create and populate filesystem image: ${IMAGE_NAME} ==="

# Check if running as root, as mount/mkfs operations require it
if [ "$(id -u)" -ne 0 ]; then
  echo "This script needs to be run with sudo or as root." >&2
  # Try to re-run with sudo
  # exec sudo "$0" "$@"
  # exit 1
fi

# Clean up previous mount point if it exists and is a directory
if [ -d "${MOUNT_POINT}" ]; then
    echo "--- Attempting to unmount stale mount point ${MOUNT_POINT}..."
    sudo umount "${MOUNT_POINT}" || true # Ignore errors if not mounted
    echo "--- Removing stale mount point directory ${MOUNT_POINT}..."
    sudo rmdir "${MOUNT_POINT}" || true # Ignore errors if already removed or contains files
fi

echo "--- Creating new filesystem image ${IMAGE_NAME} (${IMAGE_SIZE_MB}MB)..."
rm -f "${IMAGE_NAME}" # Remove old image if it exists
dd if=/dev/zero of="${IMAGE_NAME}" bs=1M count=${IMAGE_SIZE_MB} status=progress

echo "--- Formatting ${IMAGE_NAME} as ext4..."
mkfs.ext4 -F "${IMAGE_NAME}"

echo "--- Creating temporary mount point ${MOUNT_POINT}..."
sudo mkdir -p "${MOUNT_POINT}"

echo "--- Mounting ${IMAGE_NAME} to ${MOUNT_POINT}..."
sudo mount -o loop "${IMAGE_NAME}" "${MOUNT_POINT}"

# Create directories and files
echo "--- Populating the filesystem..."

sudo mkdir -p "${MOUNT_POINT}/data"
sudo mkdir -p "${MOUNT_POINT}/usr/bin"
sudo mkdir -p "${MOUNT_POINT}/etc"

echo "Hello from Echo OS! This is the root directory." | sudo tee "${MOUNT_POINT}/hello.txt" > /dev/null
echo "Some important notes for our OS." | sudo tee "${MOUNT_POINT}/data/notes.txt" > /dev/null
echo "Configuration file example." | sudo tee "${MOUNT_POINT}/etc/config.conf" > /dev/null

# Create test directory and copy ELF files from host's ./test directory
sudo mkdir -p "${MOUNT_POINT}/test"
echo "--- Copying ELF files to ${MOUNT_POINT}/test..."
# Copy hello_elf as 'hello' for the kernel test
if [ -f "./test/hello_elf" ]; then
    sudo cp "./test/hello_elf" "${MOUNT_POINT}/test/hello"
    echo "Copied ./test/hello_elf to ${MOUNT_POINT}/test/hello"
else
    echo "Error: ./test/hello_elf not found. This is required for the kernel test."
    # Consider exiting if this file is critical for the image build
    # exit 1 
fi
# Copy world_elf as 'world'
if [ -f "./test/world_elf" ]; then
    sudo cp "./test/world_elf" "${MOUNT_POINT}/test/world"
    echo "Copied ./test/world_elf to ${MOUNT_POINT}/test/world"
else
    echo "Warning: ./test/world_elf not found. Skipping copy."
fi

# Create a dummy executable (optional)
sudo touch "${MOUNT_POINT}/usr/bin/myapp"
sudo chmod +x "${MOUNT_POINT}/usr/bin/myapp"
echo '#!/bin/sh' | sudo tee "${MOUNT_POINT}/usr/bin/myapp" > /dev/null
echo 'echo "MyApp executed!"' | sudo tee -a "${MOUNT_POINT}/usr/bin/myapp" > /dev/null

echo "--- Filesystem populated. Contents:"
sudo ls -R "${MOUNT_POINT}"

echo "--- Unmounting ${IMAGE_NAME} from ${MOUNT_POINT}..."
sudo umount "${MOUNT_POINT}"

echo "--- Removing temporary mount point ${MOUNT_POINT}..."
sudo rmdir "${MOUNT_POINT}"

echo "=== Filesystem image ${IMAGE_NAME} created and populated successfully! ==="

exit 0
