import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { getGroupDetail, updateGroupTags } from '@/features/groups/api';
import type { GroupDetailResponse } from '@/features/groups/types';
import type { AppError } from '@/lib/tauri';
import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';

export function GroupDetailPage() {
  const queryClient = useQueryClient();
  const { groupName } = useParams<{ groupName: string }>();
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);
  const decodedGroupName = groupName ? decodeURIComponent(groupName) : null;
  const [tagDraft, setTagDraft] = useState('');
  const [feedback, setFeedback] = useState<string | null>(null);

  const detailQuery = useQuery<GroupDetailResponse, AppError>({
    queryKey: ['group-detail', activeClusterProfileId, groupName],
    enabled: Boolean(activeClusterProfileId && decodedGroupName),
    queryFn: () => getGroupDetail(activeClusterProfileId!, decodedGroupName!),
  });

  const tagMutation = useMutation<unknown, AppError, { clusterProfileId: string; groupName: string; tags: string[] }>({
    mutationFn: updateGroupTags,
    onSuccess: async () => {
      setFeedback('消费组标签已更新。');
      await queryClient.invalidateQueries({ queryKey: ['group-detail', activeClusterProfileId, groupName] });
      await queryClient.invalidateQueries({ queryKey: ['groups'] });
    },
    onError: (error) => setFeedback(error.message),
  });

  const handleSaveTags = () => {
    if (!activeClusterProfileId || !decodedGroupName) return;
    const tags = tagDraft.split(',').map((tag) => tag.trim()).filter(Boolean);
    tagMutation.mutate({ clusterProfileId: activeClusterProfileId, groupName: decodedGroupName, tags });
  };

  return (
    <PageFrame
      eyebrow="消费组详情"
      title={groupName ? decodeURIComponent(groupName) : '消费组详情'}
      description="从积压概览进入主题与分区级诊断。"
      contextualInfo={<div><div className="workspace-note">全局 header 已统一展示当前集群，这里只保留对象级诊断与返回动作。</div></div>}
      actions={
        <Link to="/groups" className="button-shell" data-variant="ghost">
          返回消费组
        </Link>
      }
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          {!activeClusterProfileId ? (
            <EmptyState title="请先选择活动集群" description="消费组诊断依赖当前集群配置。" />
          ) : detailQuery.isLoading ? (
            <div className="workspace-note py-6">正在读取消费组详情…</div>
          ) : detailQuery.isError ? (
            <EmptyState
              title="消费组详情加载失败"
              description={detailQuery.error.message}
              action={
                <button type="button" className="button-shell" data-variant="primary" onClick={() => detailQuery.refetch()}>
                  重试
                </button>
              }
            />
          ) : detailQuery.data ? (
            <>
              <div className="toolbar-shell mb-3">
                <div className="lg:col-span-3">
                  <div className="field-label">状态</div>
                  <div className="workspace-title">{detailQuery.data.group.state}</div>
                </div>
                <div className="lg:col-span-3">
                  <div className="field-label">总积压</div>
                  <div className="workspace-title">{detailQuery.data.group.totalLag}</div>
                </div>
                <div className="lg:col-span-3">
                  <div className="field-label">主题数</div>
                  <div className="workspace-title">{detailQuery.data.group.topicCount}</div>
                </div>
                <div className="lg:col-span-3">
                  <div className="field-label">分区数</div>
                  <div className="workspace-title">{detailQuery.data.group.partitionCount}</div>
                </div>
              </div>

              {feedback ? <div className="feedback-banner mb-3" data-tone="signal">{feedback}</div> : null}

              <div className="workspace-block">
                <div className="workspace-section-label">主题级积压</div>
                <TableShell
                  columns={['主题', '总积压', '影响分区', '操作']}
                  emptyState={<EmptyState title="暂无主题级积压" description="当前没有可展示的 topic lag breakdown。" />}
                >
                  {detailQuery.data.topicLagBreakdown.map((item) => (
                    <tr key={item.topic}>
                      <td className="font-medium text-ink">{item.topic}</td>
                      <td>{item.totalLag}</td>
                      <td>{item.partitionsImpacted}</td>
                      <td>
                        <Link to={`/topics/${encodeURIComponent(item.topic)}`} className="button-shell" data-variant="ghost">
                          打开主题
                        </Link>
                      </td>
                    </tr>
                  ))}
                </TableShell>
              </div>

              <div className="workspace-block">
                <div className="workspace-section-label">分区级积压</div>
                <TableShell
                  columns={['主题', '分区', '已提交偏移', 'Log End', 'Lag']}
                  emptyState={<EmptyState title="暂无分区级积压" description="该消费组当前没有可展示的 committed lag 数据。" />}
                >
                  {detailQuery.data.partitionLagBreakdown.map((item) => (
                    <tr key={`${item.topic}-${item.partition}`}>
                      <td className="font-medium text-ink">{item.topic}</td>
                      <td>{item.partition}</td>
                      <td className="font-mono text-xs text-ink-dim">{item.committedOffset ?? '—'}</td>
                      <td className="font-mono text-xs text-ink-dim">{item.logEndOffset ?? '—'}</td>
                      <td>{item.lag}</td>
                    </tr>
                  ))}
                </TableShell>
              </div>
            </>
          ) : null}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">摘要</div>
          {detailQuery.data ? (
            <div className="list-stack">
              <div className="list-row">
                <div>
                  <p className="list-row-title">消费组</p>
                  <p className="list-row-meta font-mono">{detailQuery.data.group.name}</p>
                </div>
                <Badge tone="signal">真实数据</Badge>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">状态</p>
                  <p className="list-row-meta">{detailQuery.data.group.state}</p>
                </div>
              </div>
              <div className="list-row">
                <div className="min-w-0 flex-1">
                  <p className="list-row-title">本地标签</p>
                  <p className="list-row-meta">
                    {detailQuery.data.group.tags.length ? detailQuery.data.group.tags.join(' · ') : '暂无标签'}
                  </p>
                  <div className="mt-3 flex flex-col gap-2">
                    <input
                      className="field-shell w-full"
                      value={tagDraft}
                      placeholder="输入逗号分隔标签，例如 prod, critical"
                      onChange={(event) => setTagDraft(event.target.value)}
                    />
                    <div className="workspace-actions">
                      <button
                        type="button"
                        className="button-shell"
                        data-variant="ghost"
                        onClick={() => setTagDraft(detailQuery.data?.group.tags.join(', ') ?? '')}
                      >
                        载入当前标签
                      </button>
                      <button
                        type="button"
                        className="button-shell"
                        data-variant="primary"
                        disabled={tagMutation.isPending}
                        onClick={handleSaveTags}
                      >
                        {tagMutation.isPending ? '保存中…' : '保存标签'}
                      </button>
                    </div>
                  </div>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">客户端主机</p>
                  <p className="list-row-meta">
                    {detailQuery.data.coordinatorInfo?.host ?? '当前 group list 未返回成员主机'}
                  </p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">客户端标识</p>
                  <p className="list-row-meta">
                    {detailQuery.data.coordinatorInfo?.brokerId ?? '当前 group list 未返回成员 client.id'}
                  </p>
                </div>
              </div>
            </div>
          ) : (
            <EmptyState title="暂无摘要" description="加载消费组详情后，这里会显示关键上下文。" />
          )}
        </aside>
      </section>
    </PageFrame>
  );
}
