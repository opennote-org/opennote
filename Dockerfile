# Runtime Stage
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

# Copy pre-built backend binary
# Ensure you have run `cargo build --release` in backend/ directory
COPY backend/target/release/notes-backend /app/notes-backend

# Copy pre-built frontend files
# Ensure you have run `flutter build web --release` in frontend/ directory
COPY frontend/build/web /usr/share/nginx/html

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
