import { invokeCommand } from '@/lib/tauri';
import type { CreateMessageBookmarkInput, ListMessageBookmarksInput, MessageBookmark } from '@/features/bookmarks/types';

export function listMessageBookmarks(request: ListMessageBookmarksInput) {
  return invokeCommand<MessageBookmark[]>('list_message_bookmarks', { request });
}

export function createMessageBookmark(request: CreateMessageBookmarkInput) {
  return invokeCommand<MessageBookmark>('create_message_bookmark', { request });
}

export function deleteMessageBookmark(id: string) {
  return invokeCommand<void>('delete_message_bookmark', { id });
}
