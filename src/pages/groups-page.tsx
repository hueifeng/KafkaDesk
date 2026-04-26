import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { listGroups } from '@/features/groups/api';
import type { GroupSummary } from '@/features/groups/types';
import type { AppError } from '@/lib/tauri';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';

type GroupSort = 'totalLag' | 'name' | 'state';

function sortGroups(items: GroupSummary[], sortBy: GroupSort) {
  const cloned = [...items];

  cloned.sort((left, right) => {
    if (sortBy === 'name') {
      return left.name.localeCompare(right.name, 'zh-CN');
    }

    if (sortBy === 'state') {
      return left.state.localeCompare(right.state, 'zh-CN') || right.totalLag - left.totalLag;
    }

    return right.totalLag - left.totalLag || left.name.localeCompare(right.name, 'zh-CN');
  });

  return cloned;
}

export function GroupsPage() {
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);
  const [query, setQuery] = useState('');
  const [laggingOnly, setLaggingOnly] = useState(true);
  const [sortBy, setSortBy] = useState<GroupSort>('totalLag');
  const [tagFilter, setTagFilter] = useState('');

  const groupsQuery = useQuery<GroupSummary[], AppError>({
    queryKey: ['groups', activeClusterProfileId, query, laggingOnly],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () =>
      listGroups({
        clusterProfileId: activeClusterProfileId!,
        query: query.trim() || undefined,
        laggingOnly,
        limit: 200,
      }),
  });

  const allTags = useMemo(
    () => Array.from(new Set((groupsQuery.data ?? []).flatMap((group) => group.tags))).sort((left, right) => left.localeCompare(right, 'zh-CN')),
    [groupsQuery.data],
  );
  const groups = useMemo(() => {
    const filtered = tagFilter
      ? (groupsQuery.data ?? []).filter((group) => group.tags.includes(tagFilter))
      : (groupsQuery.data ?? []);
    return sortGroups(filtered, sortBy);
  }, [groupsQuery.data, sortBy, tagFilter]);

  return (
    <PageFrame
      eyebrow="积压诊断"
      title="消费组"
      description="快速定位积压高、状态异常的消费组。"
      contextualInfo={<div><div className="workspace-note">筛选与积压视角切换。</div></div>}
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          <div className="workspace-toolbar">
            <div className="workspace-actions">
              <button type="button" className="button-shell" data-variant="ghost" onClick={() => setLaggingOnly((current) => !current)}>
                {laggingOnly ? '只看积压中：开' : '只看积压中：关'}
              </button>
            </div>
          </div>

          <div className="toolbar-shell mb-3">
            <div className="lg:col-span-8">
              <label className="field-label" htmlFor="groups-query">搜索</label>
              <input id="groups-query" className="field-shell w-full" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="按消费组名筛选" />
            </div>
            <div className="lg:col-span-4">
              <label className="field-label" htmlFor="groups-sort">排序</label>
              <select id="groups-sort" className="field-shell w-full" value={sortBy} onChange={(event) => setSortBy(event.target.value as GroupSort)}>
                <option value="totalLag">按积压</option>
                <option value="name">按名称</option>
                <option value="state">按状态</option>
              </select>
            </div>
            <div className="lg:col-span-4">
              <label className="field-label" htmlFor="groups-tag-filter">标签</label>
              <select id="groups-tag-filter" className="field-shell w-full" value={tagFilter} onChange={(event) => setTagFilter(event.target.value)}>
                <option value="">全部标签</option>
                {allTags.map((tag) => (
                  <option key={tag} value={tag}>{tag}</option>
                ))}
              </select>
            </div>
          </div>

          {!activeClusterProfileId ? (
            <EmptyState title="请先选择活动集群" description="消费组诊断依赖当前集群配置。" />
          ) : groupsQuery.isLoading ? (
            <div className="workspace-note py-6">正在读取消费组与积压数据…</div>
          ) : groupsQuery.isError ? (
            <EmptyState
              title="消费组加载失败"
              description={groupsQuery.error.message}
              action={
                <button type="button" className="button-shell" data-variant="primary" onClick={() => groupsQuery.refetch()}>
                  重试
                </button>
              }
            />
          ) : (
            <TableShell
              initialVisibleRowCount={50}
              rowLabel="个消费组"
              columns={['消费组', '标签', '状态', '总积压', '主题数', '分区数', '最近活动', '操作']}
              emptyState={<EmptyState title="没有匹配的消费组" description="请调整筛选条件，或确认当前集群中存在消费组。" />}
            >
              {groups.map((group) => (
                <tr key={group.name}>
                  <td className="font-medium text-ink">{group.name}</td>
                  <td>{group.tags.length ? group.tags.join(' · ') : '—'}</td>
                  <td>{group.state}</td>
                  <td>{group.totalLag}</td>
                  <td>{group.topicCount}</td>
                  <td>{group.partitionCount}</td>
                  <td>{group.lastSeenAt ?? '未知'}</td>
                  <td>
                    <Link to={`/groups/${encodeURIComponent(group.name)}`} className="button-shell" data-variant="ghost">
                      详情
                    </Link>
                  </td>
                </tr>
              ))}
            </TableShell>
          )}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">使用提示</div>
          <div className="list-stack">
            <div className="list-row">
              <div>
                <p className="list-row-title">先看总积压</p>
                <p className="list-row-meta">默认按 total lag 降序排序，优先定位最异常的消费组。</p>
              </div>
            </div>
            <div className="list-row">
              <div>
                <p className="list-row-title">再进详情</p>
                <p className="list-row-meta">详情页会给出 topic / partition 级积压分布。</p>
              </div>
            </div>
          </div>
        </aside>
      </section>
    </PageFrame>
  );
}
