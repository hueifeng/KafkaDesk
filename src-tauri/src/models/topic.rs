use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListTopicsRequest {
    pub cluster_profile_id: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub include_internal: Option<bool>,
    #[serde(default)]
    pub favorites_only: Option<bool>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GetTopicDetailRequest {
    pub cluster_profile_id: String,
    pub topic_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopicSummaryDto {
    pub name: String,
    pub partition_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replication_factor: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_hint: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopicPartitionDto {
    pub partition_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earliest_offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leader: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replica_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer_group_summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopicRelatedGroupDto {
    pub name: String,
    pub total_lag: i64,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopicConfigEntryDto {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TopicDetailResponseDto {
    pub topic: TopicSummaryDto,
    pub partitions: Vec<TopicPartitionDto>,
    pub related_groups: Vec<TopicRelatedGroupDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced_config: Option<Vec<TopicConfigEntryDto>>,
}
