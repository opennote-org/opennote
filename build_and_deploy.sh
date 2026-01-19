#!/bin/bash
set -e

DATA_DIR="/data/notes"

echo "Pulling updates from the original repo..."
git switch main
git reset --hard
git pull

echo "Starting build process..."

# Check for fvm or flutter
if command -v fvm &> /dev/null; then
    echo "fvm detected. Using fvm flutter..."
    FLUTTER_CMD="fvm flutter"
elif command -v flutter &> /dev/null; then
    echo "flutter detected. Using system flutter..."
    FLUTTER_CMD="flutter"
else
    echo "Error: Neither fvm nor flutter found in PATH."
    exit 1
fi

# --- Frontend Build ---
echo "Building Frontend..."
cd frontend/lib

# Swap constants for production
if [ -f "constants.dart" ]; then
    echo "Backing up constants.dart..."
    mv constants.dart constants.dart.bak
fi

echo "Using constants.prod.dart..."
cp constants.prod.dart constants.dart

cd .. # Go to frontend root

echo "Running $FLUTTER_CMD build web..."
$FLUTTER_CMD build web --release --pwa-strategy=none

cd lib
# Restore original constants
if [ -f "constants.dart.bak" ]; then
    echo "Restoring original constants.dart..."
    mv constants.dart.bak constants.dart
else
    # If there was no backup (shouldn't happen if logic is correct), remove the prod copy
    rm constants.dart
fi
cd ../.. # Go back to project root

# --- Backend Build ---
echo "Building Backend..."
cd backend
cargo build --release
cd ..

# --- Docker Build ---
echo "Building Docker Image..."
docker build -t notes-app .

# --- Deploy ---
CONTAINER_NAME="notes-app-container"
PORT=8085

echo "Deploying to port $PORT..."

# Stop existing container if running
if [ "$(docker ps -q -f name=$CONTAINER_NAME)" ]; then
    echo "Stopping existing container..."
    docker stop $CONTAINER_NAME
fi

# Remove existing container if exists (running or stopped)
if [ "$(docker ps -aq -f name=$CONTAINER_NAME)" ]; then
    echo "Removing existing container..."
    docker rm $CONTAINER_NAME
fi

echo "Starting new container..."
docker run -d \
    -v $DATA_DIR:/app/data \
    --name $CONTAINER_NAME \
    --restart unless-stopped \
    -p $PORT:80 \
    notes-app

echo "Deployment complete! Application is running on http://localhost:$PORT"
