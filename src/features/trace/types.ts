import type { MessageRef, TimeRange } from '@/features/messages/types';

export type TraceResultMode = 'timeline' | 'table';

export type RunTraceQueryInput = {
  clusterProfileId: string;
  keyType: string;
  keyValue: string;
  topicScope?: string[];
  timeRange: TimeRange;
  resultMode?: TraceResultMode;
};

export type TraceEvent = {
  messageRef: MessageRef;
  timestamp: string;
  topic: string;
  partition: number;
  offset: string;
  keyPreview?: string | null;
  payloadPreview?: string | null;
  matchedBy: string;
};

export type TraceQuerySummary = {
  keyType: string;
  keyValue: string;
  scannedTopics: string[];
  matchedCount: number;
  resultMode: TraceResultMode;
};

export type TraceQueryResult = {
  querySummary: TraceQuerySummary;
  events: TraceEvent[];
  timeline: TraceEvent[];
  confidenceNotes?: string[] | null;
};
