FROM rust:1.96-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    protobuf-compiler \
    cmake \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN cargo build --release --package opennote-server \
    && mv target/release/opennote-server /usr/local/bin/opennote-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/opennote-server /usr/local/bin/opennote-server

EXPOSE 8080

ENV DEFAULT_SQLITE_DATA_FOLDER_NAME="opennote_server"

# This must be set by the user when `docker run` it
ENV SERVER_PASSWORD=""

VOLUME ["/root/.config/opennote_server"]

CMD ["opennote-server"]
