import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { EmptyState } from '@/components/ui/empty-state';
import { listAuditEvents } from '@/features/audit/api';
import type { AuditEventSummary } from '@/features/audit/types';
import { listMessageBookmarks } from '@/features/bookmarks/api';
import type { MessageBookmark } from '@/features/bookmarks/types';
import { listClusters } from '@/features/clusters/api';
import type { ClusterProfileSummary } from '@/features/clusters/types';
import { listGroups } from '@/features/groups/api';
import type { GroupSummary } from '@/features/groups/types';
import { listReplayJobs } from '@/features/replay/api';
import type { ReplayJobSummary } from '@/features/replay/types';
import { listSavedQueries } from '@/features/saved-queries/api';
import type { SavedQuery } from '@/features/saved-queries/types';
import { listTopics } from '@/features/topics/api';
import type { TopicSummary } from '@/features/topics/types';
import type { AppError } from '@/lib/tauri';

function renderCountState(options: {
  isLoading: boolean;
  isError: boolean;
  dataLength?: number;
}) {
  if (options.isLoading) {
    return '…';
  }

  if (options.isError) {
    return '!';
  }

  return options.dataLength ?? 0;
}

function formatReplayStatus(status: string) {
  switch (status) {
    case 'accepted':
      return '已接受';
    case 'publishing':
      return '投递中';
    case 'delivered':
      return '已投递';
    case 'validated':
      return '已验证';
    case 'queued_local':
      return '本地排队';
    case 'succeeded':
      return '已完成';
    case 'queued':
      return '已排队';
    default:
      return status;
  }
}

export function OverviewPage() {
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);

  const clustersQuery = useQuery<ClusterProfileSummary[], AppError>({
    queryKey: ['clusters'],
    queryFn: listClusters,
  });

  const topicsQuery = useQuery<TopicSummary[], AppError>({
    queryKey: ['overview-topics', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listTopics({ clusterProfileId: activeClusterProfileId!, limit: 12 }),
  });

  const groupsQuery = useQuery<GroupSummary[], AppError>({
    queryKey: ['overview-groups', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listGroups({ clusterProfileId: activeClusterProfileId!, limit: 12 }),
  });

  const replayJobsQuery = useQuery<ReplayJobSummary[], AppError>({
    queryKey: ['overview-replay-jobs', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listReplayJobs(activeClusterProfileId!),
  });

  const auditQuery = useQuery<AuditEventSummary[], AppError>({
    queryKey: ['overview-audit', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listAuditEvents({ clusterProfileId: activeClusterProfileId || undefined, limit: 8 }),
  });

  const bookmarksQuery = useQuery<MessageBookmark[], AppError>({
    queryKey: ['overview-bookmarks', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listMessageBookmarks({ clusterProfileId: activeClusterProfileId! }),
  });

  const savedQueriesQuery = useQuery<SavedQuery[], AppError>({
    queryKey: ['overview-saved-queries'],
    queryFn: listSavedQueries,
  });

  const laggingGroups = (groupsQuery.data ?? []).filter((group) => group.totalLag > 0).sort((left, right) => right.totalLag - left.totalLag);
  const replayItems = replayJobsQuery.data?.slice(0, 3) ?? [];
  const auditItems = auditQuery.data?.slice(0, 3) ?? [];
  return (
    <PageFrame
      eyebrow="工作区概览"
      title="概览"
      description="查看当前集群的风险信号、快捷入口与最近工作上下文。"
      actions={
        <div className="workspace-actions">
          <Link to="/messages" className="button-shell" data-variant="ghost">
            去消息排查
          </Link>
          <Link to="/trace" className="button-shell" data-variant="ghost">
            去追踪
          </Link>
        </div>
      }
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          {clustersQuery.isLoading ? (
            <div className="workspace-note py-6">加载集群工作区中…</div>
          ) : clustersQuery.isError ? (
            <EmptyState
              title="集群列表加载失败"
              description={clustersQuery.error.message}
              action={
                <button type="button" className="button-shell" data-variant="primary" onClick={() => clustersQuery.refetch()}>
                  重试
                </button>
              }
            />
          ) : !clustersQuery.data?.length ? (
            <EmptyState
              title="还没有可用集群"
              description="先创建一个集群配置，概览页会自动接上主题、消费组、回放和审计数据。"
              action={<Link to="/settings/cluster-profiles" className="button-shell" data-variant="primary">前往集群配置</Link>}
            />
          ) : !activeClusterProfileId ? (
            <div className="workspace-note py-6">建立活动集群上下文中…</div>
          ) : (
            <>
              <div className="mb-4 grid gap-2 md:grid-cols-2 xl:grid-cols-4">
                <div className="h-full rounded-md border border-line bg-surface/70 p-3">
                  <div className="workspace-section-label">集群配置数</div>
                  <div className="mt-1 text-xl font-semibold text-ink">{clustersQuery.data.length}</div>
                </div>
                <div className="h-full rounded-md border border-line bg-surface/70 p-3">
                  <div className="workspace-section-label">主题</div>
                  <div className="mt-1 text-xl font-semibold text-ink">{renderCountState({ isLoading: topicsQuery.isLoading, isError: topicsQuery.isError, dataLength: topicsQuery.data?.length })}</div>
                </div>
                <div className="h-full rounded-md border border-line bg-surface/70 p-3">
                  <div className="workspace-section-label">积压组</div>
                  <div className="mt-1 text-xl font-semibold text-ink">{renderCountState({ isLoading: groupsQuery.isLoading, isError: groupsQuery.isError, dataLength: laggingGroups.length })}</div>
                </div>
                <div className="h-full rounded-md border border-line bg-surface/70 p-3">
                  <div className="workspace-section-label">已收藏消息</div>
                  <div className="mt-1 text-xl font-semibold text-ink">{renderCountState({ isLoading: bookmarksQuery.isLoading, isError: bookmarksQuery.isError, dataLength: bookmarksQuery.data?.length })}</div>
                </div>
              </div>

              <div className="form-grid">
                <div className="form-section">
                  <div className="form-section-title">高风险消费组</div>
                  <div className="list-stack mt-3">
                    {groupsQuery.isLoading ? (
                      <div className="workspace-note py-4">正在读取消费组状态…</div>
                    ) : groupsQuery.isError ? (
                      <EmptyState
                        title="消费组状态加载失败"
                        description={groupsQuery.error.message}
                        action={
                          <button type="button" className="button-shell" data-variant="primary" onClick={() => groupsQuery.refetch()}>
                            重试
                          </button>
                        }
                      />
                    ) : laggingGroups.length ? laggingGroups.slice(0, 5).map((group) => (
                      <div key={group.name} className="list-row">
                        <div>
                          <p className="list-row-title">{group.name}</p>
                          <p className="list-row-meta">总积压 {group.totalLag} · 分区 {group.partitionCount}</p>
                        </div>
                        <Link to={`/groups/${encodeURIComponent(group.name)}`} className="button-shell" data-variant="ghost">
                          查看
                        </Link>
                      </div>
                    )) : (
                      <EmptyState title="当前没有积压风险" description="未发现总积压大于 0 的消费组。" />
                    )}
                  </div>
                </div>

                <div className="form-section">
                  <div className="form-section-title">最近回放 / 审计</div>
                  <div className="list-stack mt-3">
                    {replayJobsQuery.isLoading && !replayItems.length ? <div className="workspace-note py-4">正在读取最近回放…</div> : null}
                    {replayJobsQuery.isError ? (
                      <EmptyState
                        title="最近回放加载失败"
                        description={replayJobsQuery.error.message}
                        action={
                          <button type="button" className="button-shell" data-variant="primary" onClick={() => replayJobsQuery.refetch()}>
                            重试
                          </button>
                        }
                      />
                    ) : null}
                    {replayItems.map((job) => (
                      <div key={job.id} className="list-row">
                        <div>
                          <p className="list-row-title">回放 → {job.targetTopic}</p>
                          <p className="list-row-meta">{formatReplayStatus(job.status)} · {job.createdAt}</p>
                        </div>
                        <Link to="/replay" className="button-shell" data-variant="ghost">打开</Link>
                      </div>
                    ))}
                    {auditQuery.isLoading && !auditItems.length ? <div className="workspace-note py-4">正在读取最近审计…</div> : null}
                    {auditQuery.isError ? (
                      <EmptyState
                        title="最近审计加载失败"
                        description={auditQuery.error.message}
                        action={
                          <button type="button" className="button-shell" data-variant="primary" onClick={() => auditQuery.refetch()}>
                            重试
                          </button>
                        }
                      />
                    ) : null}
                    {auditItems.map((event) => (
                      <div key={event.id} className="list-row">
                        <div>
                          <p className="list-row-title">审计 → {event.eventType}</p>
                          <p className="list-row-meta">{event.summary}</p>
                        </div>
                        <Link to="/audit" className="button-shell" data-variant="ghost">查看</Link>
                      </div>
                    ))}
                    {!replayJobsQuery.isLoading && !auditQuery.isLoading && !replayJobsQuery.isError && !auditQuery.isError && !replayItems.length && !auditItems.length ? <EmptyState title="暂无操作记录" description="执行回放或其他操作后，这里会显示最近活动。" /> : null}
                  </div>
                </div>
              </div>
            </>
          )}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">快速入口</div>
          <div className="list-stack">
            <div className="list-row">
              <div>
                <p className="list-row-title">保存的查询</p>
                <p className="list-row-meta">
                  {savedQueriesQuery.isLoading
                    ? '正在读取查询模板…'
                    : savedQueriesQuery.isError
                      ? '查询模板读取失败'
                      : `${savedQueriesQuery.data?.length ?? 0} 条查询模板`}
                </p>
              </div>
              <Link to="/saved-queries" className="button-shell" data-variant="ghost">打开</Link>
            </div>
            <div className="list-row">
              <div>
                <p className="list-row-title">主题快照</p>
                <p className="list-row-meta">
                  {topicsQuery.isLoading
                    ? '正在读取主题元数据…'
                    : topicsQuery.isError
                      ? `主题加载失败：${topicsQuery.error.message}`
                      : topicsQuery.data?.slice(0, 3).map((topic) => topic.name).join(' / ') || '暂无'}
                </p>
              </div>
            </div>
          </div>

          <div className="workspace-section-label mt-4">最近收藏消息</div>
          <div className="list-stack">
            {bookmarksQuery.isLoading ? <div className="workspace-note py-4">正在读取收藏消息…</div> : null}
            {bookmarksQuery.isError ? (
              <EmptyState
                title="收藏消息加载失败"
                description={bookmarksQuery.error.message}
                action={
                  <button type="button" className="button-shell" data-variant="primary" onClick={() => bookmarksQuery.refetch()}>
                    重试
                  </button>
                }
              />
            ) : null}
            {bookmarksQuery.data?.slice(0, 3).map((bookmark) => (
              <div key={bookmark.id} className="list-row">
                <div>
                  <p className="list-row-title font-mono">{bookmark.messageRef.topic}</p>
                  <p className="list-row-meta">
                    {bookmark.messageRef.partition} / {bookmark.messageRef.offset}
                    {bookmark.label ? ` · ${bookmark.label}` : ''}
                  </p>
                </div>
                <Link
                  to={`/messages/${encodeURIComponent(bookmark.messageRef.topic)}/${bookmark.messageRef.partition}/${encodeURIComponent(bookmark.messageRef.offset)}`}
                  className="button-shell"
                  data-variant="ghost"
                >
                  打开
                </Link>
              </div>
            ))}
            {!bookmarksQuery.isLoading && !bookmarksQuery.isError && !bookmarksQuery.data?.length ? <EmptyState title="暂无收藏消息" description="在消息详情页收藏后，这里会显示最近入口。" /> : null}
          </div>

          <div className="workspace-section-label mt-4">最近保存查询</div>
          <div className="list-stack">
            {savedQueriesQuery.isLoading ? <div className="workspace-note py-4">正在读取保存查询…</div> : null}
            {savedQueriesQuery.isError ? (
              <EmptyState
                title="保存查询加载失败"
                description={savedQueriesQuery.error.message}
                action={
                  <button type="button" className="button-shell" data-variant="primary" onClick={() => savedQueriesQuery.refetch()}>
                    重试
                  </button>
                }
              />
            ) : null}
            {savedQueriesQuery.data?.slice(0, 3).map((query) => (
              <div key={query.id} className="list-row">
                <div>
                  <p className="list-row-title">{query.name}</p>
                  <p className="list-row-meta">{query.queryType} · {query.lastRunAt ?? '未运行'}</p>
                </div>
                <Link to="/saved-queries" className="button-shell" data-variant="ghost">
                  管理
                </Link>
              </div>
            ))}
            {!savedQueriesQuery.isLoading && !savedQueriesQuery.isError && !savedQueriesQuery.data?.length ? <EmptyState title="暂无保存查询" description="从消息页保存有界查询后，这里会展示最近查询模板。" /> : null}
          </div>
        </aside>
      </section>
    </PageFrame>
  );
}
