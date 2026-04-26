import type { ValidationStage, ValidationStageStatus } from '@/lib/tauri';

export type ListTopicsInput = {
  clusterProfileId: string;
  query?: string;
  includeInternal?: boolean;
  favoritesOnly?: boolean;
  cursor?: string | null;
  limit?: number;
};

export type TopicSummary = {
  name: string;
  partitionCount: number;
  replicationFactor?: number | null;
  schemaType?: string | null;
  retentionSummary?: string | null;
  activityHint?: string | null;
  isFavorite: boolean;
};

export type TopicPartitionSummary = {
  partitionId: number;
  earliestOffset?: string | null;
  latestOffset?: string | null;
  leader?: string | null;
  replicaStatus?: string | null;
  consumerGroupSummary?: string | null;
};

export type TopicRelatedGroupSummary = {
  name: string;
  totalLag: number;
  state: string;
};

export type TopicConfigEntry = {
  key: string;
  value: string;
};

export type TopicDetailResponse = {
  topic: TopicSummary;
  partitions: TopicPartitionSummary[];
  relatedGroups: TopicRelatedGroupSummary[];
  advancedConfig?: TopicConfigEntry[] | null;
};

export type TopicOperationConfigEntry = {
  key: string;
  value?: string | null;
  isSupported: boolean;
  isReadOnly?: boolean;
  isDefault?: boolean;
  isSensitive?: boolean;
  source?: string | null;
  note?: string | null;
};

export type TopicOperationsOverviewStatus = ValidationStageStatus;

export type TopicOperationsOverviewResponse = {
  status: TopicOperationsOverviewStatus;
  message: string;
  stages: ValidationStage[];
  configEntries: TopicOperationConfigEntry[];
};

export type UpdateTopicConfigInput = {
  clusterProfileId: string;
  topicName: string;
  configKey: string;
  requestedValue?: string | null;
  expectedCurrentValue?: string | null;
  riskAcknowledged: boolean;
};

export type ExpandTopicPartitionsInput = {
  clusterProfileId: string;
  topicName: string;
  requestedPartitionCount: number;
  expectedCurrentPartitionCount: number;
  riskAcknowledged: boolean;
};

export type UpdateTopicConfigResponse = {
  topicName: string;
  configKey: string;
  previousValue?: string | null;
  requestedValue?: string | null;
  resultingValue?: string | null;
  auditRef?: string | null;
  warning?: string | null;
};

export type ExpandTopicPartitionsResponse = {
  topicName: string;
  previousPartitionCount: number;
  requestedPartitionCount: number;
  resultingPartitionCount: number;
  auditRef?: string | null;
  warning?: string | null;
};
