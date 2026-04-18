import { describe, expect, it } from 'vitest';
import { mapClusterToStorePayload, normalizeEnvironment, selectPreferredCluster } from '@/features/clusters/shared-helpers';
import type { ClusterProfileSummary } from '@/features/clusters/types';

const clusters: ClusterProfileSummary[] = [
  {
    id: 'cluster-a',
    name: '开发集群',
    environment: 'dev',
    bootstrapServers: 'localhost:9092',
    authMode: 'none',
    tlsMode: 'system-default',
    isFavorite: false,
  },
  {
    id: 'cluster-b',
    name: '生产集群',
    environment: 'prod',
    bootstrapServers: 'kafka.prod:9092',
    authMode: 'none',
    tlsMode: 'system-default',
    isFavorite: true,
  },
];

describe('cluster shared helpers', () => {
  it('normalizes unknown environments to dev', () => {
    expect(normalizeEnvironment('sandbox')).toBe('dev');
  });

  it('picks favorite cluster before first fallback', () => {
    expect(selectPreferredCluster(clusters)?.id).toBe('cluster-b');
  });

  it('maps cluster summary into store payload', () => {
    expect(mapClusterToStorePayload(clusters[1])).toEqual({
      activeClusterProfileId: 'cluster-b',
      activeClusterName: '生产集群',
      environment: 'prod',
    });
  });
});
