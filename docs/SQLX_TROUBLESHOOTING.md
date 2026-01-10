# SQLx Troubleshooting Guide for LLMs

This guide helps LLMs quickly diagnose and fix SQLx-related compilation errors in Soul Player.

## Table of Contents

1. [Quick Diagnostic Decision Tree](#quick-diagnostic-decision-tree)
2. [Common Error Patterns](#common-error-patterns)
3. [Understanding SQLx Error Messages](#understanding-sqlx-error-messages)
4. [Step-by-Step Resolution Workflows](#step-by-step-resolution-workflows)
5. [When to Regenerate Query Cache](#when-to-regenerate-query-cache)
6. [Testing After Schema Changes](#testing-after-schema-changes)

---

## Quick Diagnostic Decision Tree

```
SQLx compilation error?
│
├─ Error mentions "DATABASE_URL not found" or "connection refused"
│  └─ FIX: Check DATABASE_URL setup (see Section 2.1)
│
├─ Error mentions "no such table" or "relation does not exist"
│  └─ FIX: Run migrations (see Section 2.2)
│
├─ Error mentions "column not found" or "type mismatch"
│  ├─ Did you recently change the schema?
│  │  └─ FIX: Regenerate query cache (see Section 2.3)
│  └─ Is your query correct?
│     └─ FIX: Check query syntax and column names (see Section 2.4)
│
├─ Error mentions "macro failed" or "proc-macro panicked"
│  └─ FIX: Check query! macro syntax (see Section 2.5)
│
└─ Tests failing but compilation works
   └─ FIX: Check test database setup (see Section 2.6)
```

---

## Common Error Patterns

### 2.1 DATABASE_URL Not Found

**Error symptoms:**
```
error: DATABASE_URL not found in environment
```
or
```
thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value:
DotenvError(Io(Os { code: 2, kind: NotFound, message: "No such file or directory" }))'
```

**Root cause:** `.env` file missing or `DATABASE_URL` not set.

**Fix:**
```bash
# Check if .env exists
ls -la .env

# If missing, create from template
cp .env.example .env

# Verify DATABASE_URL is set
cat .env | grep DATABASE_URL
# Should output: DATABASE_URL=sqlite:libraries/soul-storage/.tmp/dev.db

# Create database if needed
cd libraries/soul-storage
sqlx database create
sqlx migrate run
```

**Verification:**
```bash
cargo clean
cargo check -p soul-storage
```

---

### 2.2 Migrations Not Applied

**Error symptoms:**
```
error: no such table: tracks
```
or
```
error: relation "tracks" does not exist
```
or
```
error returned from database: (code: 1) no such table: tracks
```

**Root cause:** Database schema not created or migrations not run.

**Fix:**
```bash
cd libraries/soul-storage

# Apply all migrations
sqlx migrate run --source migrations

# Verify migrations applied
sqlx migrate info --source migrations
# Should show all migrations with status "Applied"
```

**Alternative fix (clean start):**
```bash
cd libraries/soul-storage

# Delete existing database and start fresh
rm -f .tmp/dev.db

# Recreate and migrate
sqlx database create
sqlx migrate run --source migrations
```

**Verification:**
```bash
cargo clean
cargo build -p soul-storage
```

---

### 2.3 Query Cache Outdated

**Error symptoms:**
```
error: column `new_column` does not exist
```
or
```
error: type mismatch for column `some_field`
expected type `Option<i32>` but got `i32`
```
or
```
error: query returned unexpected number of columns
```

**When this happens:**
- After adding/removing columns from tables
- After changing column types
- After modifying queries in the code
- After pulling schema changes from git

**Root cause:** SQLx offline query cache (`.sqlx/` directory) doesn't match current schema.

**Fix:**
```bash
cd libraries/soul-storage

# Step 1: Apply any pending migrations
sqlx migrate run --source migrations

# Step 2: Regenerate query cache
cargo sqlx prepare -- --lib

# Step 3: Verify the .sqlx directory was updated
git status
# Should show modified files in .sqlx/

# Step 4: Clean build
cargo clean
cargo build -p soul-storage
```

**If still failing:**
```bash
cd libraries/soul-storage

# Nuclear option: delete and regenerate everything
rm -rf .sqlx/
rm -f .tmp/dev.db
sqlx database create
sqlx migrate run --source migrations
cargo sqlx prepare -- --lib
cargo clean
cargo build -p soul-storage
```

---

### 2.4 Query Syntax Errors

**Error symptoms:**
```
error: syntax error at or near "SELCT"
```
or
```
error: column "artsit_name" does not exist
Did you mean "artist_name"?
```
or
```
error: bind parameter $1 not found
```

**Root cause:** Typo or incorrect SQL syntax in `query!` macro.

**How to debug:**
1. **Find the query** in the error message - it will show the exact line number
2. **Copy the SQL** from the query! macro
3. **Test manually** against the database:
   ```bash
   cd libraries/soul-storage
   sqlite3 .tmp/dev.db
   sqlite> .schema tracks  -- Check table schema
   sqlite> SELECT id, title FROM tracks LIMIT 1;  -- Test your query
   ```
4. **Common issues:**
   - Column name typos (e.g., `artsit_name` → `artist_name`)
   - Missing table aliases in JOINs
   - Wrong number of bind parameters (`?` placeholders)
   - Incorrect SQL keywords (e.g., `SELCT` → `SELECT`)

**Fix:** Correct the query syntax in your Rust code.

---

### 2.5 Macro Syntax Errors

**Error symptoms:**
```
error: expected `,`, found keyword `as`
```
or
```
error: proc macro panicked
message: expected string literal
```
or
```
error: failed to parse query
```

**Root cause:** Incorrect usage of `query!` or `query_as!` macro.

**Common mistakes:**

1. **Missing `r#` for multi-line strings:**
   ```rust
   // ❌ WRONG
   sqlx::query!("
       SELECT id, title
       FROM tracks
   ")

   // ✅ CORRECT
   sqlx::query!(
       r#"
       SELECT id, title
       FROM tracks
       "#
   )
   ```

2. **Forgetting bind parameters:**
   ```rust
   // ❌ WRONG
   sqlx::query!("SELECT * FROM tracks WHERE id = ?")

   // ✅ CORRECT
   sqlx::query!("SELECT * FROM tracks WHERE id = ?", track_id)
   ```

3. **Using query() instead of query!():**
   ```rust
   // ❌ WRONG (runtime query, not compile-time verified)
   sqlx::query("SELECT * FROM tracks").bind(track_id)

   // ✅ CORRECT (compile-time verified)
   sqlx::query!("SELECT * FROM tracks WHERE id = ?", track_id)
   ```

---

### 2.6 Test Database Issues

**Error symptoms:**
- Compilation succeeds
- Tests fail with database errors
- "table already exists" in tests

**Root cause:** Test database not properly isolated or migrations not run in tests.

**How Soul Player tests work:**
```rust
// Tests use TestDb helper that creates isolated databases
let test_db = TestDb::new().await;  // Creates fresh in-memory DB
let pool = test_db.pool();           // Get pool reference

// Migrations are automatically run in TestDb::new()
// Each test gets a completely isolated database
```

**If tests fail:**

1. **Check TestDb usage:**
   ```rust
   #[tokio::test]
   async fn test_something() {
       let test_db = TestDb::new().await;  // ✅ Correct
       let pool = test_db.pool();

       // NOT:
       // let pool = SqlitePool::connect("...").await;  // ❌ Wrong
   }
   ```

2. **Verify test_helpers module:**
   ```bash
   # Check that test_helpers.rs exists
   ls -la libraries/soul-storage/tests/test_helpers.rs

   # Read the TestDb implementation
   cat libraries/soul-storage/tests/test_helpers.rs
   ```

3. **Run single test for debugging:**
   ```bash
   cargo test -p soul-storage --test database_integration -- test_name --nocapture
   ```

---

## Understanding SQLx Error Messages

### Error Message Anatomy

```
error: no such table: tracks
  --> libraries/soul-storage/src/tracks/mod.rs:123:5
   |
123 |     sqlx::query!(
   |     ^^^^^^^^^^^^
   |
   = note: query: SELECT id, title FROM tracks WHERE id = ?
```

**Components:**
1. **Error type:** `no such table: tracks` - the actual database error
2. **Location:** `src/tracks/mod.rs:123:5` - exact file and line number
3. **Macro invocation:** Points to the `query!` macro call
4. **Query text:** Shows the actual SQL being executed

### Reading Type Mismatch Errors

```
error[E0308]: mismatched types
  --> src/tracks/mod.rs:145:21
   |
145 |     let count: i32 = row.count;
   |                ---   ^^^^^^^^^ expected `i32`, found `Option<i64>`
```

**What this means:**
- SQLx determined the query might return `NULL` (hence `Option<i64>`)
- Your code expects a non-nullable `i32`

**Fix options:**
1. Change Rust type: `let count: Option<i64> = row.count;`
2. Use `COALESCE`: `SELECT COALESCE(COUNT(*), 0) as count`
3. Unwrap with default: `let count = row.count.unwrap_or(0) as i32;`

---

## Step-by-Step Resolution Workflows

### Workflow A: First-Time Setup

```bash
# 1. Create .env file
cp .env.example .env

# 2. Install sqlx-cli (if not already installed)
cargo install sqlx-cli --no-default-features --features sqlite

# 3. Create database
cd libraries/soul-storage
sqlx database create

# 4. Run migrations
sqlx migrate run --source migrations

# 5. Prepare offline mode
cargo sqlx prepare -- --lib

# 6. Verify
cd ../..
cargo check -p soul-storage
cargo test -p soul-storage
```

### Workflow B: After Schema Changes

```bash
# 1. Navigate to storage library
cd libraries/soul-storage

# 2. Create migration (if adding new changes)
sqlx migrate add your_migration_name
# Edit the generated file in migrations/

# 3. Apply migration
sqlx migrate run --source migrations

# 4. Update query cache
cargo sqlx prepare -- --lib

# 5. Verify
cd ../..
cargo build -p soul-storage
cargo test -p soul-storage
```

### Workflow C: After Pulling Git Changes

```bash
# 1. Check for new migrations
cd libraries/soul-storage
sqlx migrate info --source migrations
# Look for "Pending" migrations

# 2. Apply pending migrations
sqlx migrate run --source migrations

# 3. Regenerate query cache (if schema changed)
cargo sqlx prepare -- --lib

# 4. Verify
cd ../..
cargo build -p soul-storage
```

### Workflow D: Compilation Errors After Query Changes

```bash
# 1. Ensure migrations are current
cd libraries/soul-storage
sqlx migrate run --source migrations

# 2. Regenerate query cache
cargo sqlx prepare -- --lib

# 3. Clean build
cd ../..
cargo clean
cargo build -p soul-storage

# If still failing:
# 4. Check the actual query in the error message
# 5. Test query manually in sqlite3:
cd libraries/soul-storage
sqlite3 .tmp/dev.db
sqlite> .schema tracks
sqlite> [paste your query here]
sqlite> .quit

# 6. Fix the query in Rust code
# 7. Rebuild
cargo build -p soul-storage
```

---

## When to Regenerate Query Cache

**ALWAYS regenerate when:**
- ✅ Adding/removing table columns
- ✅ Changing column types (e.g., `TEXT` → `INTEGER`)
- ✅ Adding/removing tables
- ✅ Modifying SQL queries in Rust code
- ✅ After running new migrations
- ✅ After pulling schema changes from git
- ✅ CI fails with "column not found" errors

**DON'T need to regenerate when:**
- ❌ Only changing Rust code (not queries)
- ❌ Adding Rust functions that don't use queries
- ❌ Modifying test helpers (unless they use queries)
- ❌ Changing documentation or comments

**Command:**
```bash
cd libraries/soul-storage
cargo sqlx prepare -- --lib
git add .sqlx/
git commit -m "chore: update sqlx query cache"
```

---

## Testing After Schema Changes

### Unit Tests
```bash
# Run all storage tests
cargo test -p soul-storage

# Run specific test file
cargo test -p soul-storage --test database_integration

# Run single test
cargo test -p soul-storage --test database_integration -- test_track_full_lifecycle

# Show test output
cargo test -p soul-storage -- --nocapture
```

### Integration Tests Structure

Soul Player uses the vertical slice architecture. Tests are organized by feature:
- `tracks_tests.rs` - Track CRUD and search
- `playlists_tests.rs` - Playlist operations
- `database_integration.rs` - Comprehensive integration tests

**Example test pattern:**
```rust
#[tokio::test]
async fn test_something() {
    // 1. Setup isolated test database
    let test_db = TestDb::new().await;
    let pool = test_db.pool();

    // 2. Create test fixtures
    let user = create_test_user(pool, "TestUser").await;
    let track = create_test_track(pool, "Test Song", None, None, 1, Some("/test.mp3")).await;

    // 3. Test the operation
    let result = soul_storage::tracks::get_by_id(pool, track).await.unwrap();

    // 4. Assert expectations
    assert!(result.is_some());
    assert_eq!(result.unwrap().title, "Test Song");
}
```

---

## Common Pitfalls for LLMs

### Pitfall 1: Using runtime queries instead of compile-time
```rust
// ❌ WRONG - No compile-time verification
sqlx::query("SELECT * FROM tracks WHERE id = ?").bind(id)

// ✅ CORRECT - Compile-time verified
sqlx::query!("SELECT * FROM tracks WHERE id = ?", id)
```

### Pitfall 2: Forgetting to parse TrackId/PlaylistId/UserId
```rust
// ❌ WRONG - Type mismatch
sqlx::query!("SELECT * FROM tracks WHERE id = ?", track_id)

// ✅ CORRECT - Parse to i64 first
let track_id_i64: i64 = track_id.as_str().parse()?;
sqlx::query!("SELECT * FROM tracks WHERE id = ?", track_id_i64)
```

### Pitfall 3: Not handling Option types correctly
```rust
// ❌ WRONG - Assumes always present
let count: i32 = row.count;

// ✅ CORRECT - Handle NULL case
let count: i32 = row.count.unwrap_or(0);
```

### Pitfall 4: Ignoring per-user context
```rust
// ❌ WRONG - Missing user_id filter
pub async fn get_playlists(pool: &SqlitePool) -> Result<Vec<Playlist>>

// ✅ CORRECT - Always include user_id
pub async fn get_playlists(pool: &SqlitePool, user_id: UserId) -> Result<Vec<Playlist>>
```

### Pitfall 5: Using `.unwrap()` in library code
```rust
// ❌ WRONG - Panic in library
pub fn process(id: TrackId) -> i64 {
    id.as_str().parse().unwrap()
}

// ✅ CORRECT - Return Result
pub fn process(id: TrackId) -> Result<i64> {
    id.as_str().parse()
        .map_err(|_| SoulError::Storage("Invalid track ID".to_string()))
}
```

---

## Quick Reference Commands

```bash
# Check DATABASE_URL
echo $DATABASE_URL

# Check migrations status
cd libraries/soul-storage && sqlx migrate info --source migrations

# Apply migrations
cd libraries/soul-storage && sqlx migrate run --source migrations

# Regenerate query cache
cd libraries/soul-storage && cargo sqlx prepare -- --lib

# Clean build
cargo clean && cargo build -p soul-storage

# Run tests
cargo test -p soul-storage

# Run specific test file
cargo test -p soul-storage --test database_integration

# Check schema in database
cd libraries/soul-storage && sqlite3 .tmp/dev.db ".schema"

# Interactive database session
cd libraries/soul-storage && sqlite3 .tmp/dev.db
```

---

## Resources

- Main SQLx setup guide: [SQLX_SETUP.md](./SQLX_SETUP.md)
- Architecture overview: [ARCHITECTURE.md](./ARCHITECTURE.md)
- Project conventions: [CONVENTIONS.md](./CONVENTIONS.md)
- Project instructions for LLMs: [../CLAUDE.md](../CLAUDE.md)
- SQLx documentation: https://github.com/launchbadge/sqlx
- SQLx offline mode: https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md

---

**Last Updated:** 2026-01-09
**Maintainer:** Soul Player Team
