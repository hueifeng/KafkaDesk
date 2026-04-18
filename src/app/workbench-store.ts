import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

type WorkbenchStore = {
  addRecentItem: (item: { path: string; label: string }) => void;
  clearRecentItems: () => void;
  recentItems: Array<{ path: string; label: string }>;
  activeClusterProfileId: string | null;
  activeClusterName: string;
  environment: 'local' | 'dev' | 'test' | 'prod';
  searchValue: string;
  setClusterContext: (input: {
    activeClusterProfileId?: string | null;
    activeClusterName: string;
    environment: 'local' | 'dev' | 'test' | 'prod';
  }) => void;
  setSearchValue: (value: string) => void;
};

export const DEFAULT_CLUSTER_NAME = '未选择集群配置';
export const WORKBENCH_STORE_KEY = 'traceforge-workbench';

export const useWorkbenchStore = create<WorkbenchStore>()(
  persist(
    (set) => ({
      addRecentItem: (item) =>
        set((state) => {
          if (state.recentItems[0]?.path === item.path && state.recentItems[0]?.label === item.label) {
            return state;
          }

          const dedupedItems = state.recentItems.filter((i) => i.path !== item.path);
          return { recentItems: [item, ...dedupedItems].slice(0, 10) };
        }),
      clearRecentItems: () => set({ recentItems: [] }),
      recentItems: [],
      activeClusterProfileId: null,
      activeClusterName: DEFAULT_CLUSTER_NAME,
      environment: 'local',
      searchValue: '',
      setClusterContext: ({ activeClusterProfileId = null, activeClusterName, environment }) =>
        set((state) => {
          if (
            state.activeClusterProfileId === activeClusterProfileId &&
            state.activeClusterName === activeClusterName &&
            state.environment === environment
          ) {
            return state;
          }

          return { activeClusterProfileId, activeClusterName, environment };
        }),
      setSearchValue: (value) =>
        set((state) => {
          if (state.searchValue === value) {
            return state;
          }

          return { searchValue: value };
        }),
    }),
    {
      name: WORKBENCH_STORE_KEY,
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        recentItems: state.recentItems,
        activeClusterProfileId: state.activeClusterProfileId,
        activeClusterName: state.activeClusterName,
        environment: state.environment,
        searchValue: state.searchValue,
      }),
    },
  ),
);
