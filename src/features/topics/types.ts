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
