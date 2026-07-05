# OpenNote Server

Self-hosted backend server for OpenNote. Sync your notes across devices by running your own server.

## Quick Start

### Option 1: Run with Docker

```bash
docker run -d \
  --name opennote-server \
  -p 8080:8080 \
  -e SERVER_PASSWORD=your-secret-password \
  -v opennote_data:/root/.config/opennote_server \
  ghcr.io/<your-username>/opennote-server:latest
```

### Option 2: Build from Source

```bash
cargo build --release --package opennote-server
./target/release/opennote-server
```

## Configuration

The server reads `configurations.json` from `~/.config/opennote_server/` (inside the container) on first run. On macOS, it will be `/Users/yourusername/Library/Application Support/opennote_server/`. On Windows, it is `C:\Users\yourusername\AppData\Roaming\opennote_server`.

Defaults:

| Setting | Default   |
| ------- | --------- |
| Host    | `0.0.0.0` |
| Port    | `8080`    |
| Workers | `4`       |

If the host in the configuration is `localhost`, then change it to `0.0.0.0`.

### Environment Variables

| Variable                          | Description                                              | Default           |
| --------------------------------- | -------------------------------------------------------- | ----------------- |
| `SERVER_PASSWORD`                 | Password clients must send in the `Authorization` header | `""` (empty)      |
| `DEFAULT_SQLITE_DATA_FOLDER_NAME` | Folder name for SQLite data                              | `opennote_server` |

Set `SERVER_PASSWORD` to protect your server. Without it, anyone who can reach the server can access your data.

## Connect from the Desktop App

Edit `~/.config/opennote/configurations.json` and add a remote server entry under `user.remote_servers`:

On macOS, it will be `/Users/yourusername/Library/Application Support/opennote_server/`. On Windows, it is `C:\Users\yourusername\AppData\Roaming\opennote_server`.

```json
{
    "user": {
        "remote_servers": {
            "my-server": {
                "connection_string": "http://<server-ip>:8080",
                "password": "your-secret-password"
            }
        }
    }
}
```

After restarting the desktop app, a new tab will appear in the sidebar. Click it to switch to the remote server.

Note: the `password` field must match the `SERVER_PASSWORD` set on the server. If the server uses an empty password, set `"password": ""`.
