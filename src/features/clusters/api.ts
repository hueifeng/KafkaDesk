import { invokeCommand } from '@/lib/tauri';
import type {
  ClusterConnectionTestInput,
  ClusterConnectionTestResponse,
  ClusterProfile,
  ClusterProfileInput,
  ClusterProfileSummary,
  ClusterProfileUpdateInput,
} from '@/features/clusters/types';

export function listClusters() {
  return invokeCommand<ClusterProfileSummary[]>('list_clusters');
}

export function getClusterProfile(id: string) {
  return invokeCommand<ClusterProfile>('get_cluster_profile', { id });
}

export function createClusterProfile(request: ClusterProfileInput) {
  return invokeCommand<ClusterProfile>('create_cluster_profile', { request });
}

export function updateClusterProfile(request: ClusterProfileUpdateInput) {
  return invokeCommand<ClusterProfile>('update_cluster_profile', { request });
}

export function testClusterConnection(request: ClusterConnectionTestInput) {
  return invokeCommand<ClusterConnectionTestResponse>('test_cluster_connection', { request });
}
