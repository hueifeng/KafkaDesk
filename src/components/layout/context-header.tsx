import { useEffect, useMemo, useState, type FocusEvent } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useWorkbenchStore } from '@/app/workbench-store';
import { primaryNavigation, settingsNavigation, supportNavigation } from '@/app/navigation';
import { Icon } from '@/components/ui/icons';
import { listClusters } from '@/features/clusters/api';
import { mapClusterToStorePayload, selectPreferredCluster } from '@/features/clusters/shared-helpers';
import type { ClusterProfileSummary } from '@/features/clusters/types';

type EnvironmentTone = 'local' | 'warning' | 'danger';

type QuickJumpItem = {
  path: string;
  label: string;
  description: string;
};

const environmentLabels: Record<'local' | 'dev' | 'test' | 'prod', string> = {
  local: '本地',
  dev: '开发',
  test: '测试',
  prod: '生产',
};

const environmentTones: Record<'local' | 'dev' | 'test' | 'prod', EnvironmentTone> = {
  local: 'local',
  dev: 'local',
  test: 'warning',
  prod: 'danger',
};

export function ContextHeader() {
  const [isQuickJumpOpen, setIsQuickJumpOpen] = useState(false);
  const searchValue = useWorkbenchStore((state) => state.searchValue);
  const setSearchValue = useWorkbenchStore((state) => state.setSearchValue);
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);
  const activeClusterName = useWorkbenchStore((state) => state.activeClusterName);
  const environment = useWorkbenchStore((state) => state.environment);
  const setClusterContext = useWorkbenchStore((state) => state.setClusterContext);
  const isDefaultEmptyState =
    activeClusterProfileId === null && activeClusterName === '未选择集群配置' && environment === 'local';

  const clustersQuery = useQuery<ClusterProfileSummary[]>({
    queryKey: ['clusters'],
    queryFn: listClusters,
  });

  const clusters = clustersQuery.data ?? [];
  const activeCluster = useMemo(
    () => clusters.find((cluster) => cluster.id === activeClusterProfileId) ?? null,
    [activeClusterProfileId, clusters],
  );

  useEffect(() => {
    if (!isDefaultEmptyState || clusters.length === 0) {
      return;
    }

    const preferredCluster = selectPreferredCluster(clusters);
    if (!preferredCluster) {
      return;
    }

    setClusterContext(mapClusterToStorePayload(preferredCluster));
  }, [clusters, isDefaultEmptyState, setClusterContext]);

  const quickJumpItems = useMemo<QuickJumpItem[]>(
    () =>
      [...primaryNavigation, ...supportNavigation, ...settingsNavigation].map((item) => ({
        path: item.path,
        label: item.label,
        description: item.description,
      })),
    [],
  );

  const normalizedQuery = searchValue.trim().toLowerCase();
  const filteredQuickJumpItems = useMemo(() => {
    if (!normalizedQuery) {
      return quickJumpItems.slice(0, 8);
    }

    return quickJumpItems.filter((item) => {
      const haystack = `${item.label} ${item.description}`.toLowerCase();
      return haystack.includes(normalizedQuery);
    });
  }, [normalizedQuery, quickJumpItems]);

  function handleClusterChange(clusterId: string) {
    const selectedCluster = clusters.find((cluster) => cluster.id === clusterId);
    if (!selectedCluster) {
      return;
    }

    setClusterContext({
      ...mapClusterToStorePayload(selectedCluster),
    });
  }

  function handleQuickJumpBlur(event: FocusEvent<HTMLDivElement>) {
    if (event.currentTarget.contains(event.relatedTarget)) {
      return;
    }

    setIsQuickJumpOpen(false);
  }

  const environmentLabel = environmentLabels[environment];
  const environmentTone = environmentTones[environment];
  const quickJumpTitle = normalizedQuery ? '搜索结果' : '快速跳转';
  const clusterStatusMessage = clustersQuery.isLoading
    ? '正在同步可用集群，请稍候…'
    : clustersQuery.isError
      ? '集群列表加载失败，请前往设置检查连接。'
      : activeCluster
        ? `${activeCluster.bootstrapServers} · ${activeCluster.schemaRegistryProfileId ? '已关联模式注册表' : '未关联模式注册表'}`
        : clusters.length > 0
          ? '选择活动集群后即可进入主题、消费组与消息页面。'
          : '暂无可用的集群配置，请前往设置添加。';

  return (
    <header className="context-header">
      <section className="header-cluster-panel" aria-label="当前集群上下文">
        <div className="header-cluster-panel-head">
          <div className="header-cluster-heading">
            <span className="header-kicker">活动集群</span>
            <p className="header-cluster-title">{activeCluster?.name ?? activeClusterName}</p>
          </div>
          <span className="environment-badge" data-tone={environmentTone}>
            <span className="h-2 w-2 rounded-full bg-current" aria-hidden="true" />
            {environmentLabel} 环境
          </span>
        </div>

        <label className="header-field-group header-field-group-inline">
          <span className="header-field-label">切换集群</span>
          <div className="header-select-shell">
            <Icon name="cluster" className="header-field-icon" />
            <select
              className="header-select"
              value={activeClusterProfileId ?? ''}
              onChange={(event) => handleClusterChange(event.target.value)}
              disabled={clustersQuery.isLoading || clusters.length === 0}
            >
              {clusters.length === 0 ? <option value="">没有可用的集群配置</option> : null}
              {clusters.length > 0 && !activeClusterProfileId ? <option value="">选择一个集群</option> : null}
              {clusters.map((cluster) => (
                <option key={cluster.id} value={cluster.id}>
                  {cluster.name}
                </option>
              ))}
            </select>
          </div>
        </label>

        <div className="header-cluster-actions">
          <Link to="/settings/cluster-profiles" className="button-shell" data-variant="ghost">
            管理配置
          </Link>
        </div>

        <div className="header-cluster-status">{clusterStatusMessage}</div>
      </section>

      <section
        className="header-quickjump-panel"
        aria-label="页面快速跳转"
        onFocus={() => setIsQuickJumpOpen(true)}
        onBlur={handleQuickJumpBlur}
      >
        <div className="header-quickjump-inline">
          <span className="header-field-label">快速跳转</span>
          <label className="header-search-shell">
            <Icon name="search" className="header-field-icon" />
            <input
              type="search"
              className="header-search-input"
              placeholder="搜索概览、主题、设置…"
              value={searchValue}
              onChange={(event) => setSearchValue(event.target.value)}
            />
          </label>
        </div>

        {isQuickJumpOpen || normalizedQuery.length > 0 ? (
          <div className="header-quickjump-results">
            <div className="header-quickjump-results-head">
              <span>{quickJumpTitle}</span>
              <span>{filteredQuickJumpItems.length} 项</span>
            </div>

            {filteredQuickJumpItems.length > 0 ? (
              <div className="header-quickjump-list">
                {filteredQuickJumpItems.map((item) => (
                  <Link
                    key={item.path}
                    to={item.path}
                    className="header-quickjump-item"
                    onClick={() => {
                      setSearchValue('');
                      setIsQuickJumpOpen(false);
                    }}
                  >
                    <div className="header-quickjump-item-copy">
                      <span className="header-quickjump-item-title">{item.label}</span>
                      <span className="header-quickjump-item-meta">{item.description}</span>
                    </div>
                  </Link>
                ))}
              </div>
            ) : (
              <div className="header-quickjump-empty">
                未找到匹配项，请尝试搜索“概览”、“主题”或“设置”等关键词。
              </div>
            )}
          </div>
        ) : null}
      </section>
    </header>
  );
}
