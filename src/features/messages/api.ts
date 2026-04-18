import { invokeCommand } from '@/lib/tauri';
import type { MessageDetailResponse, MessageRef, MessageSummary, QueryMessagesInput } from '@/features/messages/types';

export function queryMessages(request: QueryMessagesInput) {
  return invokeCommand<MessageSummary[]>('query_messages', { request });
}

export function getMessageDetail(messageRef: MessageRef) {
  return invokeCommand<MessageDetailResponse>('get_message_detail', { request: { messageRef } });
}
