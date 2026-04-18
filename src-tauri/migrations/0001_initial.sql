CREATE TABLE IF NOT EXISTS cluster_profiles (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  environment TEXT NOT NULL,
  bootstrap_servers TEXT NOT NULL,
  auth_mode TEXT NOT NULL,
  tls_mode TEXT NOT NULL,
  schema_registry_profile_id TEXT NULL,
  notes TEXT NULL,
  tags_json TEXT NOT NULL DEFAULT '[]',
  is_favorite INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_connected_at TEXT NULL,
  is_archived INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS schema_registry_profiles (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  auth_mode TEXT NOT NULL,
  credential_ref TEXT NULL,
  notes TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_preferences (
  key TEXT PRIMARY KEY NOT NULL,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
