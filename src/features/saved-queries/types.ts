import type { HeaderFilter, OffsetRange, TimeRange } from '@/features/messages/types';

export type SavedQueryType = 'messages';

export type SavedMessagesScope = {
  topic: string;
  partitions?: number[];
  timeRange?: TimeRange;
  offsetRange?: OffsetRange;
};

export type SavedMessagesQuery = {
  keyFilter?: string;
  headerFilters?: HeaderFilter[];
  maxResults: number;
};

export type SavedQuery = {
  id: string;
  name: string;
  queryType: SavedQueryType;
  clusterProfileId: string;
  scopeJson: string;
  queryJson: string;
  description?: string | null;
  isFavorite: boolean;
  createdAt: string;
  updatedAt: string;
  lastRunAt?: string | null;
};

export type CreateSavedQueryInput = {
  name: string;
  queryType: SavedQueryType;
  clusterProfileId: string;
  scopeJson: string;
  queryJson: string;
  description?: string | null;
  isFavorite: boolean;
};

export type UpdateSavedQueryInput = CreateSavedQueryInput & {
  id: string;
  lastRunAt?: string | null;
};

export function parseSavedMessagesScope(query: SavedQuery): SavedMessagesScope | null {
  if (query.queryType !== 'messages') {
    return null;
  }

  try {
    return JSON.parse(query.scopeJson) as SavedMessagesScope;
  } catch {
    return null;
  }
}

export function parseSavedMessagesQuery(query: SavedQuery): SavedMessagesQuery | null {
  if (query.queryType !== 'messages') {
    return null;
  }

  try {
    return JSON.parse(query.queryJson) as SavedMessagesQuery;
  } catch {
    return null;
  }
}
