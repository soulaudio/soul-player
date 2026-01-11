# SQLx Development Setup

This guide explains SQLx compile-time query verification for Soul Player.

## Quick Start

### Unix/Linux/macOS

```bash
./scripts/setup-sqlx.sh
```

### Windows (PowerShell)

```powershell
# Install sqlx-cli (if not installed)
cargo install sqlx-cli --no-default-features --features sqlite

# Create database and run migrations
sqlx database create
sqlx migrate run --source libraries/soul-storage/migrations

# Prepare offline mode (optional)
cd libraries/soul-storage
cargo sqlx prepare -- --lib
cd ..\..

# Verify
cargo check -p soul-storage
```

The script will:
- Create `.env` from `.env.example` (do this manually: `copy .env.example .env`)
- Install `sqlx-cli` if needed
- Create development database at `libraries/soul-storage/.tmp/dev.db`
- Run all migrations
- Prepare offline mode (optional)
- Verify setup

## How It Works

SQLx verifies SQL queries **during compilation** by:
1. Connecting to `DATABASE_URL` (from `.env`)
2. Checking query syntax against the actual schema
3. Verifying Rust types match database types

This catches SQL bugs at compile time, not runtime.

## Daily Workflow

### Normal Development

```bash
cargo build    # Automatically verifies queries
cargo test     # Uses temp databases (isolated)
```

### After Schema Changes

**Unix/Linux/macOS:**
```bash
cd libraries/soul-storage
sqlx migrate add your_migration_name
# Edit the migration file, then:
sqlx migrate run
cargo sqlx prepare -- --lib
```

**Windows (PowerShell):**
```powershell
cd libraries/soul-storage
sqlx migrate add your_migration_name
# Edit the migration file, then:
sqlx migrate run
cargo sqlx prepare -- --lib
```

## Offline Mode (for CI/Docker)

For builds without database access:

```bash
# 1. Generate offline metadata
cd libraries/soul-storage
cargo sqlx prepare -- --lib

# 2. Commit .sqlx/ directory
git add .sqlx/
git commit -m "chore: update sqlx offline data"

# 3. Build offline
$env:SQLX_OFFLINE="true"; cargo build  # PowerShell
# or
SQLX_OFFLINE=true cargo build          # Unix/Linux/macOS
```

## Troubleshooting

### "DATABASE_URL not found"

**Windows:**
```powershell
copy .env.example .env
# Then run the setup commands above
```

**Unix/Linux/macOS:**
```bash
cp .env.example .env
./scripts/setup-sqlx.sh
```

### "relation does not exist" / "no such table"

Migrations not applied:

```bash
sqlx migrate run --source libraries/soul-storage/migrations
cargo clean && cargo build
```

### "cargo: command not found" (Git Bash on Windows)

Don't run the script with `bash` on Windows. Use PowerShell commands instead (see Quick Start above).

### Query changes not detected

```bash
cargo clean
cargo build
```

### CI failing with database errors

Update offline data:

```bash
cd libraries/soul-storage
cargo sqlx prepare -- --lib
git add .sqlx/ && git commit -m "chore: update sqlx offline data"
```

## Environment Variables

### `DATABASE_URL` (Required for compilation)

Compile-time verification database (in `.env`):

```bash
DATABASE_URL=sqlite:libraries/soul-storage/.tmp/dev.db
```

**Important**: This is ONLY for compilation, not runtime!

### `SQLX_OFFLINE` (Optional for CI)

**PowerShell:**
```powershell
$env:SQLX_OFFLINE="true"
cargo build
```

**Unix/Linux/macOS:**
```bash
SQLX_OFFLINE=true cargo build
```

### `DATABASE_PATH` (Application runtime only)

Used by Soul Player apps at runtime (NOT by SQLx):

```bash
DATABASE_PATH=/path/to/my-music.db
```

Don't confuse `DATABASE_URL` (SQLx) with `DATABASE_PATH` (runtime)!

## Database Contexts

| Context | Database | Purpose |
|---------|----------|---------|
| **Compile-time** | `.tmp/dev.db` | SQLx query verification |
| **Desktop Runtime** | App data dir | User's music library |
| **Server Runtime** | Configured path | Multi-user production |
| **Tests** | System temp dir | Isolated test databases |

## Test Database Conventions

Tests create isolated databases that are automatically cleaned up:

### How Tests Handle Databases

1. **Location**: Tests create databases in the system temp directory (e.g., `%TEMP%` on Windows, `/tmp` on Unix)
2. **Naming**: Unique names like `soul_test_{uuid}.db` prevent conflicts between parallel tests
3. **Cleanup**: Test databases are deleted after tests complete

### Writing Tests with Databases

```rust
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_with_database() {
    // Create temp database file
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path().to_str().unwrap();
    let db_url = format!("sqlite:{}", db_path);

    // Create pool and run migrations
    let pool = SqlitePool::connect(&db_url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // Run test...

    // Cleanup happens automatically when temp_file drops
}
```

### Gitignore Rules

All database files are gitignored:
- `*.db`, `*.db-shm`, `*.db-wal` - SQLite database files
- `libraries/soul-storage/.tmp/` - Compile-time verification databases

**Never commit database files to git.**

## Migration Workflow

Migrations live in `libraries/soul-storage/migrations/`:

```bash
cd libraries/soul-storage

# Create new migration
sqlx migrate add create_your_table

# Edit the generated file, then apply
sqlx migrate run

# Update offline data
cargo sqlx prepare -- --lib
```

## Multi-User Architecture

Soul Player supports multiple user contexts:

- **Desktop** (`user_id = 1`): Single default user
- **Server**: Multiple authenticated users
- **Compile-time DB**: Sample data for all user IDs

The `.tmp/dev.db` mirrors production schema for accurate verification.

## Resources

- [SQLx Documentation](https://github.com/launchbadge/sqlx)
- [SQLx Offline Mode](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#force-building-in-offline-mode)
- Soul Player Architecture: See `CLAUDE.md`

---

**Questions?** Open an issue on GitHub or check the main project documentation.
