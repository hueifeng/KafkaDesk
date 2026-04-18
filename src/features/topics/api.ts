import { invokeCommand } from '@/lib/tauri';
import type { ListTopicsInput, TopicDetailResponse, TopicSummary } from '@/features/topics/types';

export function listTopics(request: ListTopicsInput) {
  return invokeCommand<TopicSummary[]>('list_topics', { request });
}

export function getTopicDetail(clusterProfileId: string, topicName: string) {
  return invokeCommand<TopicDetailResponse>('get_topic_detail', {
    request: {
      clusterProfileId,
      topicName,
    },
  });
}
