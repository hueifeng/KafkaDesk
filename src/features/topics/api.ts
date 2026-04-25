import { invokeCommand } from '@/lib/tauri';
import type {
  ListTopicsInput,
  TopicDetailResponse,
  TopicOperationsOverviewResponse,
  TopicSummary,
  UpdateTopicConfigInput,
  UpdateTopicConfigResponse,
} from '@/features/topics/types';

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

export function getTopicOperationsOverview(clusterProfileId: string, topicName: string) {
  return invokeCommand<TopicOperationsOverviewResponse>('get_topic_operations_overview', {
    request: {
      clusterProfileId,
      topicName,
    },
  });
}

export function updateTopicConfig(request: UpdateTopicConfigInput) {
  return invokeCommand<UpdateTopicConfigResponse>('update_topic_config', { request });
}
