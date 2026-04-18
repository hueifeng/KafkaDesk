import { invokeCommand } from '@/lib/tauri';
import type { ReplayPolicy, UpdateReplayPolicyInput } from '@/features/replay-policy/types';

export function getReplayPolicy() {
  return invokeCommand<ReplayPolicy>('get_replay_policy');
}

export function updateReplayPolicy(request: UpdateReplayPolicyInput) {
  return invokeCommand<ReplayPolicy>('update_replay_policy', { request });
}
