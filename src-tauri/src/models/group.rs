use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListGroupsRequest {
    pub cluster_profile_id: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub lagging_only: Option<bool>,
    #[serde(default)]
    pub topic_filter: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetGroupDetailRequest {
    pub cluster_profile_id: String,
    pub group_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupSummaryDto {
    pub name: String,
    pub state: String,
    pub total_lag: i64,
    pub topic_count: usize,
    pub partition_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGroupTagsRequest {
    pub cluster_profile_id: String,
    pub group_name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupTopicLagDto {
    pub topic: String,
    pub total_lag: i64,
    pub partitions_impacted: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupPartitionLagDto {
    pub topic: String,
    pub partition: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub committed_offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_end_offset: Option<String>,
    pub lag: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupCoordinatorInfoDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupDetailResponseDto {
    pub group: GroupSummaryDto,
    pub topic_lag_breakdown: Vec<GroupTopicLagDto>,
    pub partition_lag_breakdown: Vec<GroupPartitionLagDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordinator_info: Option<GroupCoordinatorInfoDto>,
}
