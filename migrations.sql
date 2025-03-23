-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT 0
);

-- Create audio_files table
CREATE TABLE IF NOT EXISTS audio_files (
    id TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    mime_type TEXT NOT NULL,
    user_folder TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Create playlists table
CREATE TABLE IF NOT EXISTS playlists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Create playlist_items table
CREATE TABLE IF NOT EXISTS playlist_items (
    id TEXT PRIMARY KEY,
    playlist_id TEXT NOT NULL,
    audio_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    FOREIGN KEY (playlist_id) REFERENCES playlists(id),
    FOREIGN KEY (audio_id) REFERENCES audio_files(id)
);

-- Insert a default admin user (username: admin, password: admin)
INSERT OR IGNORE INTO users (id, username, password, is_admin) 
VALUES ('admin-user-id', 'admin', 'admin', 1);
