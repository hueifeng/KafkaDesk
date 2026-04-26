CREATE TABLE IF NOT EXISTS topic_tags (
  cluster_profile_id TEXT NOT NULL,
  topic_name TEXT NOT NULL,
  tags_json TEXT NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL,
  PRIMARY KEY (cluster_profile_id, topic_name)
);

CREATE TABLE IF NOT EXISTS group_tags (
  cluster_profile_id TEXT NOT NULL,
  group_name TEXT NOT NULL,
  tags_json TEXT NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL,
  PRIMARY KEY (cluster_profile_id, group_name)
);
