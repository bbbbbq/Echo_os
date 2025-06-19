#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

IMAGE_NAME="fs.img"
IMAGE_SIZE_MB=64
# Use a local mount point to avoid requiring root for directory creation
MOUNT_POINT="./mount_point"

SOURCE_DIR="./test"

echo "=== Preparing to create and populate filesystem image: ${IMAGE_NAME} ==="

# Check if running as root, as mount/mkfs operations require it
if [ "$(id -u)" -ne 0 ]; then
  echo "This script needs to be run with sudo or as root." >&2
  exit 1
fi

# Clean up previous mount point if it exists
if (mountpoint -q "${MOUNT_POINT}"); then
    echo "--- Attempting to unmount stale mount point ${MOUNT_POINT}..."
    umount "${MOUNT_POINT}"
fi

# Create the image file
echo "--- Creating a ${IMAGE_SIZE_MB}MB disk image: ${IMAGE_NAME}..."
dd if=/dev/zero of=${IMAGE_NAME} bs=1M count=${IMAGE_SIZE_MB} status=none

# Format the image with an ext4 filesystem
echo "--- Formatting image with ext4 filesystem..."
mkfs.ext4 -F ${IMAGE_NAME} > /dev/null

# Create mount point directory if it doesn't exist
echo "--- Creating mount point at ${MOUNT_POINT}..."
mkdir -p ${MOUNT_POINT}

# Mount the image
echo "--- Mounting ${IMAGE_NAME} to ${MOUNT_POINT}..."
mount -o loop ${IMAGE_NAME} ${MOUNT_POINT}

# Copy files from source directory to the mounted image
echo "--- Copying files from ${SOURCE_DIR} to the filesystem image..."
# Using rsync to preserve permissions and handle directories correctly
rsync -a ${SOURCE_DIR}/ ${MOUNT_POINT}/

echo "--- Filesystem contents:"
ls -lR ${MOUNT_POINT}

# Unmount the image
echo "--- Unmounting ${IMAGE_NAME}..."
umount ${MOUNT_POINT}

# The mount_point directory is kept for inspection, but it's now empty.

echo "=== Filesystem image '${IMAGE_NAME}' created and populated successfully. ==="
