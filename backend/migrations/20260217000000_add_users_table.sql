-- Create table for users
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    configuration TEXT NOT NULL
);

-- Create table for user resources
CREATE TABLE IF NOT EXISTS user_resources (
    user_id TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    PRIMARY KEY (user_id, resource_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
