-- Add folder_path to albums for strict folder-based isolation.
-- Two albums with the same title and artist but different parent folders
-- are always treated as distinct albums (no cross-folder merging).
ALTER TABLE albums ADD COLUMN folder_path TEXT NOT NULL DEFAULT '';
