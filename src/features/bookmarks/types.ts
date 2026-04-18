import type { MessageRef } from '@/features/messages/types';

export type MessageBookmark = {
  id: string;
  messageRef: MessageRef;
  label?: string | null;
  notes?: string | null;
  createdAt: string;
};

export type ListMessageBookmarksInput = {
  clusterProfileId?: string;
};

export type CreateMessageBookmarkInput = {
  messageRef: MessageRef;
  label?: string | null;
  notes?: string | null;
};
