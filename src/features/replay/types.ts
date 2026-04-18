import type { MessageHeader, MessageRef } from '@/features/messages/types';

export type CreateReplayJobInput = {
  clusterProfileId: string;
  sourceMessageRef: MessageRef;
  sourceTimestamp?: string | null;
  targetTopic: string;
  editedKey?: string | null;
  editedHeaders?: MessageHeader[] | null;
  editedPayload?: string | null;
  dryRun: boolean;
  riskAcknowledged: boolean;
};

export type ReplayJobSummary = {
  id: string;
  status: string;
  mode: string;
  targetTopic: string;
  sourceTopic: string;
  sourcePartition: number;
  sourceOffset: string;
  sourceTimestamp?: string | null;
  createdAt: string;
  startedAt?: string | null;
  completedAt?: string | null;
  riskLevel: string;
  errorMessage?: string | null;
  resultSummaryJson?: string | null;
  payloadEditJson?: string | null;
  headersEditJson?: string | null;
  keyEditJson?: string | null;
};

export type ReplayJobEvent = {
  id: string;
  eventType: string;
  eventPayloadJson?: string | null;
  createdAt: string;
};

export type ReplayJobDetailResponse = {
  job: ReplayJobSummary;
  eventHistory: ReplayJobEvent[];
  auditRef?: string | null;
};
