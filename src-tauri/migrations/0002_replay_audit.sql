CREATE TABLE IF NOT EXISTS replay_jobs (
  id TEXT PRIMARY KEY NOT NULL,
  cluster_profile_id TEXT NOT NULL,
  source_topic TEXT NOT NULL,
  source_partition INTEGER NOT NULL,
  source_offset INTEGER NOT NULL,
  source_timestamp TEXT NULL,
  target_topic TEXT NOT NULL,
  status TEXT NOT NULL,
  mode TEXT NOT NULL,
  payload_edit_json TEXT NULL,
  headers_edit_json TEXT NULL,
  key_edit_json TEXT NULL,
  dry_run INTEGER NOT NULL DEFAULT 0,
  requested_by_profile TEXT NULL,
  risk_level TEXT NOT NULL,
  created_at TEXT NOT NULL,
  started_at TEXT NULL,
  completed_at TEXT NULL,
  error_message TEXT NULL,
  result_summary_json TEXT NULL
);

CREATE TABLE IF NOT EXISTS replay_job_events (
  id TEXT PRIMARY KEY NOT NULL,
  replay_job_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  event_payload_json TEXT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_events (
  id TEXT PRIMARY KEY NOT NULL,
  event_type TEXT NOT NULL,
  target_type TEXT NOT NULL,
  target_ref TEXT NULL,
  actor_profile TEXT NULL,
  cluster_profile_id TEXT NULL,
  outcome TEXT NOT NULL,
  summary TEXT NOT NULL,
  details_json TEXT NULL,
  created_at TEXT NOT NULL
);
