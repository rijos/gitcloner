# Git Cloner

A Rust web application for managing and syncing git repositories with a simple HTML/JavaScript frontend.

## Features

- **Web-based repository management**: Add, remove, and sync git repositories through a simple web interface
- **Automatic daily synchronization**: Repositories are automatically synced once per day at 2 AM
- **Safe synchronization**: Local changes are preserved - remote changes won't override local history
- **Authentication**: Simple username/password protection stored in SQLite
- **User administration**: Command-line tool for managing users
- **No Node.js dependency**: Pure HTML/JavaScript frontend with no build tools required

## User Management

The application includes a command-line administration tool (`gitc`) for managing users:

```bash
# Add a new user
gitc add <username> <password>

# Update user password
gitc update <username> <new_password>

# Remove a user
gitc remove <username>

# List all users
gitc list
```

**Note**: No default users are created. You must create at least one user before accessing the web interface.

### Creating Your First User

After installation, create an admin user:
```bash
cargo run --bin gitc add admin your_secure_password
```

## Installation & Setup

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone and run the application**:
   ```bash
   git clone <your-repo-url>
   cd gitcloner
   cargo run
   ```

3. **Create your first user**:
   ```bash
   cargo run --bin gitc add admin your_secure_password
   ```

4. **Access the application**:
   Open your browser and navigate to `http://localhost:3030`

## Usage

1. **Create users** (if not done during setup):
   ```bash
   cargo run --bin gitc add username password
   ```

2. **Login** with your created credentials
3. **Add repositories** by entering the git URL in the form
4. **Manage repositories**:
   - **Sync**: Manually trigger a sync for any repository
   - **Remove**: Delete a repository from the list (local files will remain)

## Configuration

### Environment Variables

- `DATABASE_URL`: SQLite database path (default: `sqlite:./gitcloner.db`)

### Repository Storage

All cloned repositories are stored in the `./repos` directory by default.

## Security Features

- **Password hashing**: Uses bcrypt for secure password storage
- **Session management**: Token-based authentication with in-memory session storage
- **Safe git operations**: Preserves local changes during sync operations

## API Endpoints

### Authentication
- `POST /api/auth/login` - Login with username/password
- `POST /api/auth/logout` - Logout current session

### Repositories
- `GET /api/repositories` - List all repositories
- `POST /api/repositories` - Add a new repository
- `DELETE /api/repositories/{url}` - Remove a repository
- `POST /api/repositories/{url}/sync` - Sync a specific repository

## Development

### Database Schema

The application uses SQLite with the following tables:
- `users`: User authentication data
- `repositories`: Repository information and sync status

### Git Synchronization Strategy

The application implements a safe synchronization strategy:
1. Fetch remote changes without merging
2. Check for local modifications
3. Only perform fast-forward merges if no local changes exist
4. Preserve local history in case of conflicts

### Scheduled Tasks

Daily synchronization runs at 2 AM using tokio-cron-scheduler. The sync process:
1. Fetches all repositories from the database
2. Attempts to sync each repository
3. Updates repository status and last sync time

## Building for Production

```bash
# Build the main application
cargo build --release

# Build the admin tool
cargo build --release --bin gitc
```

The binaries will be available at:
- Main application: `target/release/gitcloner`
- Admin tool: `target/release/gitc`

### Production Deployment

1. Copy both binaries to your server
2. Set up the database: `DATABASE_URL=sqlite:/path/to/production.db`
3. Create your first user: `./gitc add admin secure_password`
4. Run the application: `./gitcloner`

## Directory Structure

```
gitcloner/
├── src/
│   ├── main.rs          # Application entry point
│   ├── auth.rs          # Authentication management
│   ├── database.rs      # Database operations
│   ├── git_manager.rs   # Git operations
│   ├── handlers.rs      # HTTP request handlers
│   ├── models.rs        # Data structures
│   └── bin/
│       └── gitc.rs      # User administration tool
├── static/
│   ├── index.html       # Frontend HTML
│   └── app.js          # Frontend JavaScript
├── migrations/
│   └── 001_initial.sql  # Database schema
├── repos/              # Cloned repositories (auto-created)
├── Cargo.toml          # Rust dependencies
└── README.md           # This file
```

## License

MIT License - see LICENSE file for details.