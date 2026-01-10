-- Background fingerprinting queue
-- Tracks audio files that need Chromaprint fingerprinting

CREATE TABLE IF NOT EXISTS fingerprint_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    -- Higher priority = processed first (0 = normal, positive = higher priority)
    priority INTEGER NOT NULL DEFAULT 0,
    -- Number of processing attempts (for retry logic)
    attempts INTEGER NOT NULL DEFAULT 0,
    -- Last error message if processing failed
    last_error TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    UNIQUE(track_id)
);

CREATE INDEX IF NOT EXISTS idx_fingerprint_queue_priority
    ON fingerprint_queue(priority DESC, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_fingerprint_queue_attempts
    ON fingerprint_queue(attempts) WHERE attempts < 3;
