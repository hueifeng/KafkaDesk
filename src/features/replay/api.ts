import { invokeCommand } from '@/lib/tauri';
import type { CreateReplayJobInput, ReplayJobDetailResponse, ReplayJobSummary } from '@/features/replay/types';

export function createReplayJob(request: CreateReplayJobInput) {
  return invokeCommand<ReplayJobDetailResponse>('create_replay_job', { request });
}

export function listReplayJobs(clusterProfileId: string) {
  return invokeCommand<ReplayJobSummary[]>('list_replay_jobs', { clusterProfileId });
}

export function getReplayJob(id: string) {
  return invokeCommand<ReplayJobDetailResponse>('get_replay_job', { id });
}
