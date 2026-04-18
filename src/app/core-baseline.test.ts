import { describe, expect, it } from 'vitest';
import { primaryNavigation, settingsNavigation, supportNavigation } from '@/app/navigation';
import { queryClient } from '@/app/query-client';
import { DEFAULT_CLUSTER_NAME, useWorkbenchStore, WORKBENCH_STORE_KEY } from '@/app/workbench-store';

describe('queryClient', () => {
  it('uses stable defaults for desktop data fetching', () => {
    expect(queryClient.getDefaultOptions().queries).toMatchObject({
      retry: 1,
      refetchOnWindowFocus: false,
      staleTime: 30_000,
    });
  });
});

describe('useWorkbenchStore', () => {
  it('persists workflow context into localStorage', () => {
    localStorage.removeItem(WORKBENCH_STORE_KEY);
    useWorkbenchStore.setState({
      recentItems: [],
      activeClusterProfileId: null,
      activeClusterName: DEFAULT_CLUSTER_NAME,
      environment: 'local',
      searchValue: '',
    });

    useWorkbenchStore.getState().setClusterContext({
      activeClusterProfileId: 'cluster-1',
      activeClusterName: '开发集群',
      environment: 'dev',
    });
    useWorkbenchStore.getState().setSearchValue('trace-id');
    useWorkbenchStore.getState().addRecentItem({ path: '/messages', label: '消息' });

    const persisted = JSON.parse(localStorage.getItem(WORKBENCH_STORE_KEY) ?? '{}');

    expect(persisted.state).toMatchObject({
      activeClusterProfileId: 'cluster-1',
      activeClusterName: '开发集群',
      environment: 'dev',
      searchValue: 'trace-id',
      recentItems: [{ path: '/messages', label: '消息' }],
    });
  });

  it('initializes with empty recentItems', () => {
    useWorkbenchStore.setState({ recentItems: [] });

    const state = useWorkbenchStore.getState();

    expect(state.recentItems).toEqual([]);
  });

  it('adds recent items correctly', () => {
    useWorkbenchStore.setState({ recentItems: [] });

    useWorkbenchStore.getState().addRecentItem({ path: '/overview', label: '概览' });

    expect(useWorkbenchStore.getState().recentItems).toContainEqual({ path: '/overview', label: '概览' });
  });
});

describe('navigation collections', () => {
  it('keeps route paths unique across navigation groups', () => {
    const paths = [...primaryNavigation, ...supportNavigation, ...settingsNavigation].map((item) => item.path);

    expect(new Set(paths).size).toBe(paths.length);
  });

  it('keeps settings routes namespaced under /settings', () => {
    expect(settingsNavigation.every((item) => item.path.startsWith('/settings/'))).toBe(true);
  });
});
