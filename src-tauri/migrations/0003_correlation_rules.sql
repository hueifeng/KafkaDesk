CREATE TABLE IF NOT EXISTS correlation_rules (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  cluster_profile_id TEXT NOT NULL,
  is_enabled INTEGER NOT NULL DEFAULT 1,
  match_strategy TEXT NOT NULL,
  scope_json TEXT NOT NULL,
  rule_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
