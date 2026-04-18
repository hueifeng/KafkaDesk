import { Link, useParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { getTopicDetail } from '@/features/topics/api';
import type { TopicDetailResponse } from '@/features/topics/types';
import type { AppError } from '@/lib/tauri';
import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';

export function TopicDetailPage() {
  const { topicName } = useParams<{ topicName: string }>();
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);

  const detailQuery = useQuery<TopicDetailResponse, AppError>({
    queryKey: ['topic-detail', activeClusterProfileId, topicName],
    enabled: Boolean(activeClusterProfileId && topicName),
    queryFn: () => getTopicDetail(activeClusterProfileId!, decodeURIComponent(topicName!)),
  });

  return (
    <PageFrame
      eyebrow="主题详情"
      title={topicName ? decodeURIComponent(topicName) : '主题详情'}
      description="查看真实分区元数据，并继续进入消息排查。"
      contextualInfo={<div><div className="workspace-note">当前集群与环境由全局 header 统一提供，详情页只保留返回与对象摘要。</div></div>}
      actions={
        <Link to="/topics" className="button-shell" data-variant="ghost">
          返回主题列表
        </Link>
      }
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          {!activeClusterProfileId ? (
            <EmptyState title="请先选择活动集群" description="主题详情依赖当前集群配置。" />
          ) : detailQuery.isLoading ? (
            <div className="workspace-note py-6">正在读取主题详情…</div>
          ) : detailQuery.isError ? (
            <EmptyState
              title="主题详情加载失败"
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
                <div className="lg:col-span-4">
                  <div className="field-label">分区数</div>
                  <div className="workspace-title">{detailQuery.data.topic.partitionCount}</div>
                </div>
                <div className="lg:col-span-4">
                  <div className="field-label">副本数</div>
                  <div className="workspace-title">{detailQuery.data.topic.replicationFactor ?? '—'}</div>
                </div>
                <div className="lg:col-span-4">
                  <div className="field-label">Schema / Retention</div>
                  <div className="workspace-note">
                    {detailQuery.data.topic.schemaType ?? '未知'} / {detailQuery.data.topic.retentionSummary ?? '暂未读取'}
                  </div>
                </div>
              </div>

              <TableShell columns={['分区', '最早偏移', '最新偏移', 'Leader', '副本状态', '消费组']}>
                {detailQuery.data.partitions.map((partition) => (
                  <tr key={partition.partitionId}>
                    <td>{partition.partitionId}</td>
                    <td className="font-mono text-xs text-ink-dim">{partition.earliestOffset ?? '—'}</td>
                    <td className="font-mono text-xs text-ink-dim">{partition.latestOffset ?? '—'}</td>
                    <td>{partition.leader ?? '—'}</td>
                    <td>{partition.replicaStatus ?? '—'}</td>
                      <td>{partition.consumerGroupSummary ?? '当前没有关联消费组'}</td>
                  </tr>
                ))}
              </TableShell>
            </>
          ) : null}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">摘要</div>
          {detailQuery.data ? (
            <div className="list-stack">
              <div className="list-row">
                <div>
                  <p className="list-row-title">主题</p>
                  <p className="list-row-meta font-mono">{detailQuery.data.topic.name}</p>
                </div>
                <Badge tone="signal">真实数据</Badge>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">活跃度</p>
                  <p className="list-row-meta">{detailQuery.data.topic.activityHint ?? '暂无'}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">相关消费组</p>
                  <p className="list-row-meta">
                    {detailQuery.data.relatedGroups.length
                      ? detailQuery.data.relatedGroups
                          .slice(0, 2)
                          .map((group) => `${group.name} (${group.state})`)
                          .join(' · ')
                      : '当前没有关联消费组'}
                  </p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">高级配置</p>
                  <p className="list-row-meta">
                    {detailQuery.data.advancedConfig?.map((item) => `${item.key}: ${item.value}`).join(' · ') || '暂无'}
                  </p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">下一步</p>
                  <p className="list-row-meta">后续将从这里进入消息查询与消费组诊断。</p>
                </div>
              </div>
            </div>
          ) : (
            <EmptyState title="暂无摘要" description="加载主题详情后，这里会显示关键上下文。" />
          )}
        </aside>
      </section>
    </PageFrame>
  );
}
