CREATE TABLE IF NOT EXISTS message_bookmarks (
  id TEXT PRIMARY KEY NOT NULL,
  cluster_profile_id TEXT NOT NULL,
  topic TEXT NOT NULL,
  partition INTEGER NOT NULL,
  offset INTEGER NOT NULL,
  label TEXT NULL,
  notes TEXT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_message_bookmarks_cluster_created_at
ON message_bookmarks (cluster_profile_id, created_at DESC);
