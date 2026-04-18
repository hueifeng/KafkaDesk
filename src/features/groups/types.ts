export type ListGroupsInput = {
  clusterProfileId: string;
  query?: string;
  laggingOnly?: boolean;
  topicFilter?: string;
  cursor?: string | null;
  limit?: number;
};

export type GroupSummary = {
  name: string;
  state: string;
  totalLag: number;
  topicCount: number;
  partitionCount: number;
  lastSeenAt?: string | null;
};

export type GroupTopicLagItem = {
  topic: string;
  totalLag: number;
  partitionsImpacted: number;
};

export type GroupPartitionLagItem = {
  topic: string;
  partition: number;
  committedOffset?: string | null;
  logEndOffset?: string | null;
  lag: number;
};

export type GroupCoordinatorInfo = {
  brokerId?: string | null;
  host?: string | null;
};

export type GroupDetailResponse = {
  group: GroupSummary;
  topicLagBreakdown: GroupTopicLagItem[];
  partitionLagBreakdown: GroupPartitionLagItem[];
  coordinatorInfo?: GroupCoordinatorInfo | null;
};
