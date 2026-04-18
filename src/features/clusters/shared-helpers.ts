import type { ClusterProfileSummary } from './types';

export function normalizeEnvironment(value: string): 'local' | 'dev' | 'test' | 'prod' {
  if (value === 'local' || value === 'dev' || value === 'test' || value === 'prod') {
    return value;
  }

  return 'dev';
}

export function selectPreferredCluster(clusters: ClusterProfileSummary[]): ClusterProfileSummary | null {
  if (!clusters.length) return null;

  return clusters.find((cluster) => cluster.isFavorite) ?? clusters[0];
}

export function mapClusterToStorePayload(cluster: ClusterProfileSummary): {
  activeClusterProfileId: string;
  activeClusterName: string;
  environment: 'local' | 'dev' | 'test' | 'prod';
} {
  return {
    activeClusterProfileId: cluster.id,
    activeClusterName: cluster.name,
    environment: normalizeEnvironment(cluster.environment),
  };
}