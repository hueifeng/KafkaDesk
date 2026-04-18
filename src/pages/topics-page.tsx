import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { listTopics } from '@/features/topics/api';
import type { TopicSummary } from '@/features/topics/types';
import type { AppError } from '@/lib/tauri';
import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';

type TopicSort = 'name' | 'partitionCount' | 'activityHint';

function sortTopics(items: TopicSummary[], sortBy: TopicSort) {
  const cloned = [...items];

  cloned.sort((left, right) => {
    if (sortBy === 'partitionCount') {
      return right.partitionCount - left.partitionCount || left.name.localeCompare(right.name, 'zh-CN');
    }

    if (sortBy === 'activityHint') {
      return (right.activityHint ?? '').localeCompare(left.activityHint ?? '', 'zh-CN') || left.name.localeCompare(right.name, 'zh-CN');
    }

    return left.name.localeCompare(right.name, 'zh-CN');
  });

  return cloned;
}

export function TopicsPage() {
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);
  const [query, setQuery] = useState('');
  const [includeInternal, setIncludeInternal] = useState(false);
  const [sortBy, setSortBy] = useState<TopicSort>('name');
  const [selectedTopicName, setSelectedTopicName] = useState<string | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);

  const topicsQuery = useQuery<TopicSummary[], AppError>({
    queryKey: ['topics', activeClusterProfileId, query, includeInternal],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () =>
      listTopics({
        clusterProfileId: activeClusterProfileId!,
        query: query.trim() || undefined,
        includeInternal,
        favoritesOnly: false,
        limit: 500,
      }),
  });

  const sortedTopics = useMemo(() => sortTopics(topicsQuery.data ?? [], sortBy), [sortBy, topicsQuery.data]);
  const visibleTopics = useMemo(() => sortedTopics.slice(0, 50), [sortedTopics]);
  const selectedTopic = useMemo(
    () => sortedTopics.find((topic) => topic.name === selectedTopicName) ?? null,
    [selectedTopicName, sortedTopics],
  );

  useEffect(() => {
    if (!sortedTopics.length) {
      setSelectedTopicName(null);
      return;
    }

    const selectedTopicIsVisible = selectedTopicName ? visibleTopics.some((topic) => topic.name === selectedTopicName) : false;
    if (!selectedTopicIsVisible) {
      setSelectedTopicName(visibleTopics[0].name);
    }
  }, [selectedTopicName, sortedTopics, visibleTopics]);

  async function handleCopyTopicName(name: string) {
    try {
      await navigator.clipboard.writeText(name);
      setFeedback(`已复制：${name}`);
    } catch {
      setFeedback('复制失败，请手动复制。');
    }
  }

  return (
    <PageFrame
      eyebrow="主题浏览"
      title="主题"
      description="快速浏览与筛选主题。"
      contextualInfo={<div><div className="workspace-note">主题筛选与预览反馈。</div></div>}
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          <div className="workspace-toolbar">
            <div className="workspace-actions">
              <button
                type="button"
                className="button-shell"
                data-variant="ghost"
                onClick={() => setIncludeInternal((current) => !current)}
              >
                {includeInternal ? '内部主题：开' : '内部主题：关'}
              </button>
            </div>
          </div>

          <div className="toolbar-shell mb-3">
            <div className="lg:col-span-8">
              <label className="field-label" htmlFor="topics-query">搜索</label>
              <input
                id="topics-query"
                className="field-shell w-full"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="请输入主题名以筛选"
              />
            </div>
            <div className="lg:col-span-4">
              <label className="field-label" htmlFor="topics-sort">排序</label>
              <select id="topics-sort" className="field-shell w-full" value={sortBy} onChange={(event) => setSortBy(event.target.value as TopicSort)}>
                <option value="name">名称</option>
                <option value="partitionCount">分区数</option>
                <option value="activityHint">活跃度</option>
              </select>
            </div>
          </div>

          {feedback ? (
            <div className="feedback-banner mb-3" data-tone="signal">
              {feedback}
            </div>
          ) : null}

          {!activeClusterProfileId ? (
            <EmptyState title="请先选择活动集群" description="主题列表依赖当前集群配置。" />
          ) : topicsQuery.isLoading ? (
            <div className="workspace-note py-6">正在读取主题元数据…</div>
          ) : topicsQuery.isError ? (
            <EmptyState
              title="加载失败"
              description={topicsQuery.error.message}
              action={
                <button type="button" className="button-shell" data-variant="primary" onClick={() => topicsQuery.refetch()}>
                  重试
                </button>
              }
            />
          ) : (
            <TableShell
              initialVisibleRowCount={50}
              rowLabel="个主题"
              columns={['主题名称', '分区', '副本', 'Schema', 'Retention', '活跃度', '操作']}
              emptyState={<EmptyState title="没有匹配结果" description="请调整筛选条件。" />}
            >
              {sortedTopics.map((topic) => {
                const active = selectedTopicName === topic.name;

                return (
                  <tr key={topic.name} className={active ? 'bg-elevated/70' : undefined}>
                    <td>
                      <button
                        type="button"
                        className="w-full text-left text-ink transition hover:text-ink"
                        onClick={() => setSelectedTopicName(topic.name)}
                      >
                        <span className="font-medium">{topic.name}</span>
                      </button>
                    </td>
                    <td>{topic.partitionCount}</td>
                    <td>{topic.replicationFactor ?? '—'}</td>
                    <td>{topic.schemaType ?? '未知'}</td>
                    <td>{topic.retentionSummary ?? '—'}</td>
                    <td>{topic.activityHint ?? '—'}</td>
                    <td>
                      <div className="flex flex-wrap gap-2">
                        <button type="button" className="button-shell" data-variant="ghost" onClick={() => setSelectedTopicName(topic.name)}>
                          预览
                        </button>
                        <Link to={`/topics/${encodeURIComponent(topic.name)}`} className="button-shell" data-variant="ghost">
                          详情
                        </Link>
                        <button
                          type="button"
                          className="button-shell"
                          data-variant="ghost"
                          onClick={() => {
                            void handleCopyTopicName(topic.name);
                          }}
                        >
                          复制
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </TableShell>
          )}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">主题预览</div>
          {!activeClusterProfileId ? (
            <EmptyState title="没有活动集群" description="先完成集群配置。" />
          ) : selectedTopic ? (
            <div className="list-stack">
              <div className="list-row">
                <div>
                  <p className="list-row-title">主题</p>
                  <p className="list-row-meta font-mono">{selectedTopic.name}</p>
                </div>
                <Badge tone="signal">已选中</Badge>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">分区 / 副本</p>
                  <p className="list-row-meta">
                    {selectedTopic.partitionCount} / {selectedTopic.replicationFactor ?? '—'}
                  </p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">Schema / Retention</p>
                  <p className="list-row-meta">
                    {selectedTopic.schemaType ?? '未识别'} / {selectedTopic.retentionSummary ?? '—'}
                  </p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">活跃度</p>
                  <p className="list-row-meta">{selectedTopic.activityHint ?? '暂无'}</p>
                </div>
              </div>
            </div>
          ) : (
            <EmptyState title="没有预览对象" description="从左侧列表选择一个主题。" />
          )}
        </aside>
      </section>
    </PageFrame>
  );
}
