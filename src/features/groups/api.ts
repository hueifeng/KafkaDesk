import { invokeCommand } from '@/lib/tauri';
import type { GroupDetailResponse, GroupSummary, ListGroupsInput, UpdateGroupTagsInput } from '@/features/groups/types';

export function listGroups(request: ListGroupsInput) {
  return invokeCommand<GroupSummary[]>('list_groups', { request });
}

export function getGroupDetail(clusterProfileId: string, groupName: string) {
  return invokeCommand<GroupDetailResponse>('get_group_detail', {
    request: {
      clusterProfileId,
      groupName,
    },
  });
}

export function updateGroupTags(request: UpdateGroupTagsInput) {
  return invokeCommand<GroupSummary>('update_group_tags', { request });
}
