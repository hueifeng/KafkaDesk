export type MessageRef = {
  clusterProfileId: string;
  topic: string;
  partition: number;
  offset: string;
};

export type HeaderFilter = {
  key: string;
  value?: string;
};

export type TimeRange = {
  start: string;
  end: string;
};

export type OffsetRange = {
  startOffset?: string;
  endOffset?: string;
};

export type QueryMessagesInput = {
  clusterProfileId: string;
  topic: string;
  partitions?: number[];
  timeRange?: TimeRange;
  offsetRange?: OffsetRange;
  keyFilter?: string;
  headerFilters?: HeaderFilter[];
  maxResults: number;
};

export type MessageSummary = {
  messageRef: MessageRef;
  timestamp: string;
  partition: number;
  offset: string;
  keyPreview?: string | null;
  decodeStatus: string;
  payloadPreview?: string | null;
};

export type MessageHeader = {
  key: string;
  value: string;
};

export type MessageDetailResponse = {
  messageRef: MessageRef;
  timestamp: string;
  keyRaw?: string | null;
  headers: MessageHeader[];
  payloadRaw: string;
  payloadDecoded?: string | null;
  decodeStatus: string;
  schemaInfo?: string | null;
  relatedHints?: string[] | null;
};
