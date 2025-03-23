# Home Audio Server

A self-hosted audio streaming server built with Rust and Actix Web, designed for personal use to manage and stream your audio collection.

## Features

- **User Management**: Create and manage user accounts with admin privileges
- **Audio File Management**: Upload, stream, and delete audio files
- **Playlist Support**: Create playlists and add/remove audio files
- **Secure API**: JWT-based authentication and HTTPS support
- **Rate Limiting**: Prevents abuse by limiting request rates

## Requirements

- Rust 1.70+
- SQLite
- OpenSSL (for TLS certificates)

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/0xRogu/home-audio.git
   cd home-audio
   ```

2. Set up environment variables:
   ```bash
   echo "SECRET_KEY=your_secure_secret_key" > .env
   ```

3. Build and run the application:
   ```bash
   cargo build --release
   ./target/release/home-audio
   ```

The server will start on `http://127.0.0.1:8080` by default.

## API Endpoints

### Authentication
- `POST /login` - Authenticate and receive a JWT token

### Audio Management
- `POST /audio` - Upload an audio file
- `GET /audio/{id}` - Stream an audio file
- `DELETE /audio/{id}` - Delete an audio file
- `GET /users/{id}/audio` - Get all audio files for a user

### Playlist Management
- `POST /playlists` - Create a new playlist
- `GET /playlists` - Get all playlists
- `GET /playlists/{id}` - Get a specific playlist
- `DELETE /playlists/{id}` - Delete a playlist
- `POST /playlists/{id}/items` - Add an audio file to a playlist
- `DELETE /playlists/{id}/items/{item_id}` - Remove an audio file from a playlist

### User Management
- `POST /users` - Create a new user
- `GET /users` - List all users
- `DELETE /users/{id}` - Delete a user

## Security

The application supports TLS for secure communication. SSL certificates are automatically generated if they don't exist.

## Development

### Running Tests
```bash
cargo test
```

### Linting
```bash
cargo clippy -- -D warnings
cargo fmt --all -- --check
```

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
