#!/bin/bash
set -e

DATA_DIR="/data/notes"
CONTAINER_NAME="notes-app-container"
PORT=8085
MCP_PORT=8086

echo "Pulling updates from the original repo..."
git switch main
git reset --hard
git pull

echo "Building Docker Image (using multi-stage build)..."
docker build -t notes-app .

echo "Deploying to port $PORT and MCP service to $MCP_PORT..."

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
    -p $MCP_PORT:8081 \
    notes-app

echo "Deployment complete!"
echo "Web App: http://localhost:$PORT"
echo "MCP Endpoint: http://localhost:$MCP_PORT/mcp"
