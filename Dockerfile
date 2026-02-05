# Stage 1: Build Frontend
FROM ghcr.io/cirruslabs/flutter:stable AS frontend-builder
WORKDIR /app
COPY frontend/ .
# Swap constants for production if the file exists
RUN if [ -f lib/constants.prod.dart ]; then \
      cp lib/constants.prod.dart lib/constants.dart; \
    fi
RUN flutter build web --release --pwa-strategy=none

# Stage 2: Build Backend
FROM rust:latest AS backend-builder
WORKDIR /app
COPY backend/ .
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    nginx \
    supervisor \
    ca-certificates \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy backend binary from builder
COPY --from=backend-builder /app/target/release/notes-backend /app/notes-backend

# Copy frontend files from builder
COPY --from=frontend-builder /app/build/web /usr/share/nginx/html

# Create data directories
RUN mkdir -p /app/data

# Copy configuration
COPY backend/config.prod.json /app/config.json

# Setup Nginx
RUN rm /etc/nginx/sites-enabled/default
COPY deployment/nginx.conf /etc/nginx/conf.d/default.conf

# Setup Supervisor
COPY deployment/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Expose port 80 (web) and 8081 (mcp server)
EXPOSE 80 8081

# Start Supervisor
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
