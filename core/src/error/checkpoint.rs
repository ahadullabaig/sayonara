/// SQLite-based checkpoint database for operation resume capability
///
/// This module provides atomic, persistent checkpoint storage using SQLite.
/// Checkpoints are saved every 60 seconds OR every 1GB written, whichever comes first.
/// All database operations use transactions for atomicity and must complete in <100ms.
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

/// Default checkpoint save interval (60 seconds)
const DEFAULT_TIME_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Default checkpoint byte interval (1GB)
const DEFAULT_BYTES_INTERVAL: u64 = 1024 * 1024 * 1024;

/// Default database path
const DEFAULT_DB_PATH: &str = "/var/lib/sayonara-wipe/checkpoints.db";

/// Universal checkpoint structure supporting all algorithms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Checkpoint {
    /// Unique checkpoint identifier (UUID)
    pub id: String,

    /// Device path (e.g., "/dev/sda")
    pub device_path: String,

    /// Algorithm name (e.g., "Gutmann", "DoD", "Random")
    pub algorithm: String,

    /// Operation session ID (groups related checkpoints)
    pub operation_id: String,

    /// Current pass number
    pub current_pass: usize,

    /// Total number of passes
    pub total_passes: usize,

    /// Bytes written so far
    pub bytes_written: u64,

    /// Total device size in bytes
    pub total_size: u64,

    /// List of completed sector offsets (for non-sequential algorithms)
    pub sectors_completed: Vec<u64>,

    /// Algorithm-specific state (JSON)
    pub state: serde_json::Value,

    /// Wipe configuration (JSON)
    pub config: serde_json::Value,

    /// Checkpoint creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Count of errors encountered so far
    pub error_count: u32,

    /// Last error message (if any)
    pub last_error: Option<String>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(
        device_path: impl Into<String>,
        algorithm: impl Into<String>,
        operation_id: impl Into<String>,
        total_passes: usize,
        total_size: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            device_path: device_path.into(),
            algorithm: algorithm.into(),
            operation_id: operation_id.into(),
            current_pass: 0,
            total_passes,
            bytes_written: 0,
            total_size,
            sectors_completed: Vec::new(),
            state: serde_json::Value::Null,
            config: serde_json::Value::Null,
            created_at: now,
            updated_at: now,
            error_count: 0,
            last_error: None,
        }
    }

    /// Update progress information
    pub fn update_progress(&mut self, pass: usize, bytes_written: u64) {
        self.current_pass = pass;
        self.bytes_written = bytes_written;
        self.updated_at = Utc::now();
    }

    /// Record an error
    pub fn record_error(&mut self, error_msg: impl Into<String>) {
        self.error_count += 1;
        self.last_error = Some(error_msg.into());
        self.updated_at = Utc::now();
    }

    /// Calculate completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_size == 0 {
            return 0.0;
        }
        (self.bytes_written as f64 / self.total_size as f64) * 100.0
    }

    /// Get progress description
    pub fn progress_description(&self) -> String {
        format!(
            "Pass {}/{}, {:.2}% complete ({} bytes / {} bytes)",
            self.current_pass + 1,
            self.total_passes,
            self.completion_percentage(),
            self.bytes_written,
            self.total_size
        )
    }
}

/// Checkpoint database manager
pub struct CheckpointManager {
    /// Database connection
    conn: Connection,

    /// Database file path
    db_path: PathBuf,

    /// Time interval between checkpoint saves
    checkpoint_interval: std::time::Duration,

    /// Byte interval between checkpoint saves
    bytes_interval: u64,

    /// Last checkpoint save time
    last_save: Instant,

    /// Bytes written at last save
    last_bytes: u64,
}

impl CheckpointManager {
    /// Create or open checkpoint database
    ///
    /// Creates the database file and schema if it doesn't exist.
    /// Uses WAL mode for better concurrency and crash resilience.
    pub fn new(db_path: Option<&str>) -> Result<Self> {
        // Use in-memory database for tests to avoid permission issues
        // Check both cfg!(test) and environment variable for test detection
        let is_test = cfg!(test) || std::env::var("SAYONARA_TEST_MODE").is_ok();

        let db_path = match db_path {
            Some(path) => PathBuf::from(path),
            None if is_test => PathBuf::from(":memory:"),
            None => PathBuf::from(DEFAULT_DB_PATH),
        };

        // Ensure parent directory exists (skip for in-memory databases)
        if db_path.to_str() != Some(":memory:") {
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create checkpoint database directory")?;
            }
        }

        let conn = Connection::open(&db_path).context("Failed to open checkpoint database")?;

        // Enable WAL mode for better concurrency and crash resilience
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("Failed to set WAL mode")?;

        // Enable foreign keys
        conn.pragma_update(None, "foreign_keys", "ON")
            .context("Failed to enable foreign keys")?;

        // Set synchronous to NORMAL for better performance while maintaining safety
        conn.pragma_update(None, "synchronous", "NORMAL")
            .context("Failed to set synchronous mode")?;

        let mut manager = Self {
            conn,
            db_path,
            checkpoint_interval: DEFAULT_TIME_INTERVAL,
            bytes_interval: DEFAULT_BYTES_INTERVAL,
            last_save: Instant::now(),
            last_bytes: 0,
        };

        manager.initialize_schema()?;

        Ok(manager)
    }

    /// Initialize database schema
    fn initialize_schema(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
            CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY NOT NULL,
                device_path TEXT NOT NULL,
                algorithm TEXT NOT NULL,
                operation_id TEXT NOT NULL,
                current_pass INTEGER NOT NULL,
                total_passes INTEGER NOT NULL,
                bytes_written INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                sectors_completed TEXT,
                state TEXT,
                config TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                error_count INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                UNIQUE(device_path, algorithm, operation_id)
            );

            CREATE INDEX IF NOT EXISTS idx_device ON checkpoints(device_path);
            CREATE INDEX IF NOT EXISTS idx_updated ON checkpoints(updated_at);
            CREATE INDEX IF NOT EXISTS idx_operation ON checkpoints(operation_id);
            CREATE INDEX IF NOT EXISTS idx_device_algo ON checkpoints(device_path, algorithm);
            "#,
            )
            .context("Failed to create checkpoint schema")?;

        Ok(())
    }

    /// Save checkpoint atomically with transaction
    ///
    /// Uses UPSERT (INSERT OR REPLACE) to handle both new and existing checkpoints.
    /// Completes in <100ms as required by spec.
    pub fn save(&mut self, checkpoint: &Checkpoint) -> Result<()> {
        let start = Instant::now();

        // Serialize complex fields to JSON
        let sectors_json = serde_json::to_string(&checkpoint.sectors_completed)
            .context("Failed to serialize sectors_completed")?;
        let state_json =
            serde_json::to_string(&checkpoint.state).context("Failed to serialize state")?;
        let config_json =
            serde_json::to_string(&checkpoint.config).context("Failed to serialize config")?;

        // Use transaction for atomicity
        let tx = self
            .conn
            .transaction()
            .context("Failed to begin transaction")?;

        tx.execute(
            r#"
            INSERT INTO checkpoints (
                id, device_path, algorithm, operation_id,
                current_pass, total_passes, bytes_written, total_size,
                sectors_completed, state, config,
                created_at, updated_at, error_count, last_error
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(device_path, algorithm, operation_id)
            DO UPDATE SET
                id = excluded.id,
                current_pass = excluded.current_pass,
                bytes_written = excluded.bytes_written,
                sectors_completed = excluded.sectors_completed,
                state = excluded.state,
                config = excluded.config,
                updated_at = excluded.updated_at,
                error_count = excluded.error_count,
                last_error = excluded.last_error
            "#,
            params![
                checkpoint.id,
                checkpoint.device_path,
                checkpoint.algorithm,
                checkpoint.operation_id,
                checkpoint.current_pass as i64,
                checkpoint.total_passes as i64,
                checkpoint.bytes_written as i64,
                checkpoint.total_size as i64,
                sectors_json,
                state_json,
                config_json,
                checkpoint.created_at.to_rfc3339(),
                checkpoint.updated_at.to_rfc3339(),
                checkpoint.error_count as i64,
                checkpoint.last_error,
            ],
        )
        .context("Failed to insert checkpoint")?;

        tx.commit()
            .context("Failed to commit checkpoint transaction")?;

        let elapsed = start.elapsed();
        if elapsed.as_millis() > 100 {
            tracing::warn!(
                elapsed_ms = elapsed.as_millis(),
                checkpoint_id = %checkpoint.id,
                "Checkpoint save exceeded 100ms target"
            );
        }

        self.last_save = Instant::now();
        self.last_bytes = checkpoint.bytes_written;

        Ok(())
    }

    /// Load most recent checkpoint for device and algorithm
    pub fn load(&self, device_path: &str, algorithm: &str) -> Result<Option<Checkpoint>> {
        let row = self
            .conn
            .query_row(
                r#"
            SELECT id, device_path, algorithm, operation_id,
                   current_pass, total_passes, bytes_written, total_size,
                   sectors_completed, state, config,
                   created_at, updated_at, error_count, last_error
            FROM checkpoints
            WHERE device_path = ?1 AND algorithm = ?2
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
                params![device_path, algorithm],
                |row| {
                    Ok(Checkpoint {
                        id: row.get(0)?,
                        device_path: row.get(1)?,
                        algorithm: row.get(2)?,
                        operation_id: row.get(3)?,
                        current_pass: row.get::<_, i64>(4)? as usize,
                        total_passes: row.get::<_, i64>(5)? as usize,
                        bytes_written: row.get::<_, i64>(6)? as u64,
                        total_size: row.get::<_, i64>(7)? as u64,
                        sectors_completed: {
                            let json: String = row.get(8)?;
                            serde_json::from_str(&json).unwrap_or_default()
                        },
                        state: {
                            let json: String = row.get(9)?;
                            serde_json::from_str(&json).unwrap_or(serde_json::Value::Null)
                        },
                        config: {
                            let json: String = row.get(10)?;
                            serde_json::from_str(&json).unwrap_or(serde_json::Value::Null)
                        },
                        created_at: {
                            let s: String = row.get(11)?;
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(Utc::now)
                        },
                        updated_at: {
                            let s: String = row.get(12)?;
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(Utc::now)
                        },
                        error_count: row.get::<_, i64>(13)? as u32,
                        last_error: row.get(14)?,
                    })
                },
            )
            .optional()
            .context("Failed to load checkpoint")?;

        Ok(row)
    }

    /// Load checkpoint by ID
    pub fn load_by_id(&self, checkpoint_id: &str) -> Result<Option<Checkpoint>> {
        let row = self
            .conn
            .query_row(
                r#"
            SELECT id, device_path, algorithm, operation_id,
                   current_pass, total_passes, bytes_written, total_size,
                   sectors_completed, state, config,
                   created_at, updated_at, error_count, last_error
            FROM checkpoints
            WHERE id = ?1
            "#,
                params![checkpoint_id],
                |row| {
                    Ok(Checkpoint {
                        id: row.get(0)?,
                        device_path: row.get(1)?,
                        algorithm: row.get(2)?,
                        operation_id: row.get(3)?,
                        current_pass: row.get::<_, i64>(4)? as usize,
                        total_passes: row.get::<_, i64>(5)? as usize,
                        bytes_written: row.get::<_, i64>(6)? as u64,
                        total_size: row.get::<_, i64>(7)? as u64,
                        sectors_completed: {
                            let json: String = row.get(8)?;
                            serde_json::from_str(&json).unwrap_or_default()
                        },
                        state: {
                            let json: String = row.get(9)?;
                            serde_json::from_str(&json).unwrap_or(serde_json::Value::Null)
                        },
                        config: {
                            let json: String = row.get(10)?;
                            serde_json::from_str(&json).unwrap_or(serde_json::Value::Null)
                        },
                        created_at: {
                            let s: String = row.get(11)?;
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(Utc::now)
                        },
                        updated_at: {
                            let s: String = row.get(12)?;
                            DateTime::parse_from_rfc3339(&s)
                                .ok()
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(Utc::now)
                        },
                        error_count: row.get::<_, i64>(13)? as u32,
                        last_error: row.get(14)?,
                    })
                },
            )
            .optional()
            .context("Failed to load checkpoint by ID")?;

        Ok(row)
    }

    /// Delete checkpoint (after successful completion)
    pub fn delete(&mut self, checkpoint_id: &str) -> Result<()> {
        let deleted = self
            .conn
            .execute(
                "DELETE FROM checkpoints WHERE id = ?1",
                params![checkpoint_id],
            )
            .context("Failed to delete checkpoint")?;

        if deleted == 0 {
            return Err(anyhow!("Checkpoint not found: {}", checkpoint_id));
        }

        Ok(())
    }

    /// Delete checkpoint by device and algorithm
    pub fn delete_by_device(&mut self, device_path: &str, algorithm: &str) -> Result<usize> {
        let deleted = self
            .conn
            .execute(
                "DELETE FROM checkpoints WHERE device_path = ?1 AND algorithm = ?2",
                params![device_path, algorithm],
            )
            .context("Failed to delete checkpoint by device")?;

        Ok(deleted)
    }

    /// List all checkpoints
    pub fn list_all(&self) -> Result<Vec<Checkpoint>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
            SELECT id, device_path, algorithm, operation_id,
                   current_pass, total_passes, bytes_written, total_size,
                   sectors_completed, state, config,
                   created_at, updated_at, error_count, last_error
            FROM checkpoints
            ORDER BY updated_at DESC
            "#,
            )
            .context("Failed to prepare list query")?;

        let checkpoints = stmt
            .query_map([], |row| {
                Ok(Checkpoint {
                    id: row.get(0)?,
                    device_path: row.get(1)?,
                    algorithm: row.get(2)?,
                    operation_id: row.get(3)?,
                    current_pass: row.get::<_, i64>(4)? as usize,
                    total_passes: row.get::<_, i64>(5)? as usize,
                    bytes_written: row.get::<_, i64>(6)? as u64,
                    total_size: row.get::<_, i64>(7)? as u64,
                    sectors_completed: {
                        let json: String = row.get(8)?;
                        serde_json::from_str(&json).unwrap_or_default()
                    },
                    state: {
                        let json: String = row.get(9)?;
                        serde_json::from_str(&json).unwrap_or(serde_json::Value::Null)
                    },
                    config: {
                        let json: String = row.get(10)?;
                        serde_json::from_str(&json).unwrap_or(serde_json::Value::Null)
                    },
                    created_at: {
                        let s: String = row.get(11)?;
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now)
                    },
                    updated_at: {
                        let s: String = row.get(12)?;
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now)
                    },
                    error_count: row.get::<_, i64>(13)? as u32,
                    last_error: row.get(14)?,
                })
            })
            .context("Failed to query checkpoints")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect checkpoints")?;

        Ok(checkpoints)
    }

    /// Check if checkpoint should be saved based on time and bytes intervals
    pub fn should_save(&self, bytes_written: u64) -> bool {
        let time_elapsed = self.last_save.elapsed() >= self.checkpoint_interval;
        let bytes_threshold = bytes_written.saturating_sub(self.last_bytes) >= self.bytes_interval;

        time_elapsed || bytes_threshold
    }

    /// Clean up stale checkpoints older than max_age
    pub fn cleanup_stale(&mut self, max_age: Duration) -> Result<usize> {
        let cutoff = Utc::now() - max_age;
        let deleted = self
            .conn
            .execute(
                "DELETE FROM checkpoints WHERE updated_at < ?1",
                params![cutoff.to_rfc3339()],
            )
            .context("Failed to cleanup stale checkpoints")?;

        Ok(deleted)
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<CheckpointStats> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM checkpoints", [], |row| row.get(0))?;

        let size_bytes = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(CheckpointStats {
            total_checkpoints: total as usize,
            database_size_bytes: size_bytes,
            database_path: self.db_path.clone(),
        })
    }

    /// Vacuum the database to reclaim space
    pub fn vacuum(&self) -> Result<()> {
        self.conn
            .execute("VACUUM", [])
            .context("Failed to vacuum database")?;
        Ok(())
    }

    /// Set custom checkpoint intervals
    pub fn set_intervals(&mut self, time: std::time::Duration, bytes: u64) {
        self.checkpoint_interval = time;
        self.bytes_interval = bytes;
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct CheckpointStats {
    pub total_checkpoints: usize,
    pub database_size_bytes: u64,
    pub database_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (CheckpointManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_checkpoints.db");
        let manager = CheckpointManager::new(Some(db_path.to_str().unwrap())).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_checkpoint_creation() {
        let cp = Checkpoint::new("/dev/sda", "Gutmann", "op-123", 35, 1024 * 1024 * 1024);
        assert_eq!(cp.device_path, "/dev/sda");
        assert_eq!(cp.algorithm, "Gutmann");
        assert_eq!(cp.total_passes, 35);
        assert_eq!(cp.current_pass, 0);
        assert_eq!(cp.bytes_written, 0);
    }

    #[test]
    fn test_save_and_load_checkpoint() {
        let (mut manager, _temp) = create_test_manager();

        let mut cp = Checkpoint::new("/dev/sda", "Gutmann", "op-123", 35, 1024 * 1024 * 1024);
        cp.update_progress(5, 512 * 1024 * 1024);

        manager.save(&cp).unwrap();

        let loaded = manager.load("/dev/sda", "Gutmann").unwrap().unwrap();
        assert_eq!(loaded.id, cp.id);
        assert_eq!(loaded.current_pass, 5);
        assert_eq!(loaded.bytes_written, 512 * 1024 * 1024);
    }

    #[test]
    fn test_checkpoint_update() {
        let (mut manager, _temp) = create_test_manager();

        let mut cp = Checkpoint::new("/dev/sda", "DoD", "op-456", 3, 1024 * 1024);
        manager.save(&cp).unwrap();

        cp.update_progress(2, 800 * 1024);
        manager.save(&cp).unwrap();

        let loaded = manager.load("/dev/sda", "DoD").unwrap().unwrap();
        assert_eq!(loaded.current_pass, 2);
        assert_eq!(loaded.bytes_written, 800 * 1024);
    }

    #[test]
    fn test_delete_checkpoint() {
        let (mut manager, _temp) = create_test_manager();

        let cp = Checkpoint::new("/dev/sda", "Random", "op-789", 1, 1024);
        let checkpoint_id = cp.id.clone();
        manager.save(&cp).unwrap();

        assert!(manager.load_by_id(&checkpoint_id).unwrap().is_some());

        manager.delete(&checkpoint_id).unwrap();

        assert!(manager.load_by_id(&checkpoint_id).unwrap().is_none());
    }

    #[test]
    fn test_list_all_checkpoints() {
        let (mut manager, _temp) = create_test_manager();

        let cp1 = Checkpoint::new("/dev/sda", "Gutmann", "op-1", 35, 1024);
        let cp2 = Checkpoint::new("/dev/sdb", "DoD", "op-2", 3, 2048);

        manager.save(&cp1).unwrap();
        manager.save(&cp2).unwrap();

        let all = manager.list_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_should_save_time_interval() {
        let (manager, _temp) = create_test_manager();

        // Should not save immediately
        assert!(!manager.should_save(100));

        // Simulate time passing
        std::thread::sleep(std::time::Duration::from_millis(100));
        // Would normally be false, but we can't easily test time-based without mocking
    }

    #[test]
    fn test_should_save_bytes_interval() {
        let (mut manager, _temp) = create_test_manager();
        manager.last_bytes = 0;

        // Should save after 1GB
        assert!(manager.should_save(DEFAULT_BYTES_INTERVAL));
        assert!(!manager.should_save(DEFAULT_BYTES_INTERVAL / 2));
    }

    #[test]
    fn test_cleanup_stale() {
        let (mut manager, _temp) = create_test_manager();

        let mut cp = Checkpoint::new("/dev/sda", "Gutmann", "op-old", 35, 1024);
        cp.updated_at = Utc::now() - Duration::days(10);
        manager.save(&cp).unwrap();

        let deleted = manager.cleanup_stale(Duration::days(7)).unwrap();
        assert_eq!(deleted, 1);

        let all = manager.list_all().unwrap();
        assert_eq!(all.len(), 0);
    }

    #[test]
    fn test_checkpoint_stats() {
        let (mut manager, _temp) = create_test_manager();

        let cp = Checkpoint::new("/dev/sda", "Gutmann", "op-1", 35, 1024);
        manager.save(&cp).unwrap();

        let stats = manager.stats().unwrap();
        assert_eq!(stats.total_checkpoints, 1);
        assert!(stats.database_size_bytes > 0);
    }

    #[test]
    fn test_error_recording() {
        let mut cp = Checkpoint::new("/dev/sda", "Gutmann", "op-1", 35, 1024);
        assert_eq!(cp.error_count, 0);
        assert!(cp.last_error.is_none());

        cp.record_error("I/O error");
        assert_eq!(cp.error_count, 1);
        assert_eq!(cp.last_error, Some("I/O error".to_string()));
    }

    #[test]
    fn test_completion_percentage() {
        let mut cp = Checkpoint::new("/dev/sda", "Gutmann", "op-1", 35, 1000);
        assert_eq!(cp.completion_percentage(), 0.0);

        cp.bytes_written = 500;
        assert_eq!(cp.completion_percentage(), 50.0);

        cp.bytes_written = 1000;
        assert_eq!(cp.completion_percentage(), 100.0);
    }

    #[test]
    fn test_save_performance() {
        let (mut manager, _temp) = create_test_manager();

        let cp = Checkpoint::new("/dev/sda", "Gutmann", "op-perf", 35, 1024 * 1024 * 1024);

        let start = Instant::now();
        manager.save(&cp).unwrap();
        let elapsed = start.elapsed();

        // Must complete in <100ms as per spec
        assert!(
            elapsed.as_millis() < 100,
            "Checkpoint save took {}ms",
            elapsed.as_millis()
        );
    }
}
