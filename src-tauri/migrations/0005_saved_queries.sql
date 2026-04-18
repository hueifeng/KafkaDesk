CREATE TABLE IF NOT EXISTS saved_queries (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  query_type TEXT NOT NULL,
  cluster_profile_id TEXT NOT NULL,
  scope_json TEXT NOT NULL,
  query_json TEXT NOT NULL,
  description TEXT NULL,
  is_favorite INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_run_at TEXT NULL
);

CREATE INDEX IF NOT EXISTS idx_saved_queries_cluster_updated_at
ON saved_queries (cluster_profile_id, updated_at DESC);
