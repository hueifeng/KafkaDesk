import { Link, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { createMessageBookmark, deleteMessageBookmark, listMessageBookmarks } from '@/features/bookmarks/api';
import type { MessageBookmark } from '@/features/bookmarks/types';
import { getMessageDetail } from '@/features/messages/api';
import type { MessageDetailResponse } from '@/features/messages/types';
import type { AppError } from '@/lib/tauri';
import { EmptyState } from '@/components/ui/empty-state';
import { Badge } from '@/components/ui/badge';
import { DecodeStatusLegend } from '@/components/messages/decode-status-legend';
import { describeDecodeStatus, formatDecodeStatus } from '@/features/messages/decode-status';

function decodedPayloadFallback(status: string) {
  switch (status) {
    case 'avro-decoded':
      return '当前没有可展示的解码结果';
    case 'schema-unsupported':
      return '已识别到 Schema Registry 关联，但当前 schemaType 尚未纳入运行时解码支持。';
    case 'schema-auth-unsupported':
      return '已识别到 Schema Registry 编码消息，但当前版本尚未实现带认证凭据的运行时解码。';
    case 'schema-auth-failed':
      return '已识别到需要认证的 Schema Registry，但当前 keyring 中没有可用 secret，或 secret 格式不正确。';
    case 'schema-references-unsupported':
      return '当前版本尚未解析带 references 的 Avro schema。';
    case 'schema-registry-error':
      return '已检测到受 Schema Registry 管理的消息，但读取 schema 元数据失败。';
    case 'schema-decode-failed':
      return 'Schema Registry 已返回 schema，但当前消息未能成功解码。';
    case 'utf8':
      return '消息体可按 UTF-8 文本查看，但当前没有 Schema Registry 解码结果。';
    case 'binary':
      return '消息体是二进制数据；当前没有可直接展示的结构化解码结果。';
    case 'empty':
      return '消息体为空。';
    default:
      return '当前没有可展示的解码结果';
  }
}

export function MessageDetailPage() {
  const { topic, partition, offset } = useParams<{ topic: string; partition: string; offset: string }>();
  const queryClient = useQueryClient();
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);

  const detailQuery = useQuery<MessageDetailResponse, AppError>({
    queryKey: ['message-detail', activeClusterProfileId, topic, partition, offset],
    enabled: Boolean(activeClusterProfileId && topic && partition && offset),
    queryFn: () =>
      getMessageDetail({
        clusterProfileId: activeClusterProfileId!,
        topic: decodeURIComponent(topic!),
        partition: Number(partition),
        offset: decodeURIComponent(offset!),
      }),
  });

  const bookmarksQuery = useQuery<MessageBookmark[], AppError>({
    queryKey: ['message-bookmarks', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listMessageBookmarks({ clusterProfileId: activeClusterProfileId! }),
  });

  const traceLaunch = detailQuery.data
    ? (() => {
        const traceHeader = detailQuery.data.headers.find((header) => /trace|request|order/i.test(header.key) && header.value.trim());
        if (traceHeader) {
          return {
            label: `按 Header / ${traceHeader.key} 追踪`,
            href: `/trace?keyType=${encodeURIComponent(`header:${traceHeader.key}`)}&keyValue=${encodeURIComponent(traceHeader.value)}&topicScope=${encodeURIComponent(detailQuery.data.messageRef.topic)}`,
          };
        }

        if (detailQuery.data.keyRaw?.trim()) {
          return {
            label: '按消息 Key 追踪',
            href: `/trace?keyType=message-key&keyValue=${encodeURIComponent(detailQuery.data.keyRaw)}&topicScope=${encodeURIComponent(detailQuery.data.messageRef.topic)}`,
          };
        }

        return null;
      })()
    : null;

  const activeBookmark = detailQuery.data
    ? (bookmarksQuery.data ?? []).find(
        (bookmark) =>
          bookmark.messageRef.clusterProfileId === detailQuery.data.messageRef.clusterProfileId &&
          bookmark.messageRef.topic === detailQuery.data.messageRef.topic &&
          bookmark.messageRef.partition === detailQuery.data.messageRef.partition &&
          bookmark.messageRef.offset === detailQuery.data.messageRef.offset,
      ) ?? null
    : null;

  const createBookmarkMutation = useMutation({
    mutationFn: createMessageBookmark,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['message-bookmarks', activeClusterProfileId] });
    },
  });

  const deleteBookmarkMutation = useMutation({
    mutationFn: deleteMessageBookmark,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['message-bookmarks', activeClusterProfileId] });
    },
  });

  const isBookmarkBusy = createBookmarkMutation.isPending || deleteBookmarkMutation.isPending;

  return (
    <PageFrame
      eyebrow="消息详情"
      title={topic ? decodeURIComponent(topic) : '消息详情'}
      description="查看消息原始内容、Header、解码状态与上下文信息。"
      contextualInfo={<div><div className="workspace-note">全局 header 已固定当前集群，这里只保留消息上下文与返回动作。</div></div>}
      actions={
        <Link to="/messages" className="button-shell" data-variant="ghost">
          返回消息查询
        </Link>
      }
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          {!activeClusterProfileId ? (
            <EmptyState title="请先选择活动集群" description="消息详情依赖当前集群配置。" />
          ) : detailQuery.isLoading ? (
            <div className="workspace-note py-6">正在读取消息详情…</div>
          ) : detailQuery.isError ? (
            <EmptyState
              title="消息详情加载失败"
              description={detailQuery.error.message}
              action={
                <button type="button" className="button-shell" data-variant="primary" onClick={() => detailQuery.refetch()}>
                  重试
                </button>
              }
            />
          ) : detailQuery.data ? (
            <div className="list-stack">
              <div className="list-row">
                <div>
                  <p className="list-row-title">消息引用</p>
                  <p className="list-row-meta font-mono">
                    {detailQuery.data.messageRef.topic} / {detailQuery.data.messageRef.partition} / {detailQuery.data.messageRef.offset}
                  </p>
                </div>
                <Badge tone="signal">真实数据</Badge>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">解码状态</p>
                  <p className="list-row-meta">{formatDecodeStatus(detailQuery.data.decodeStatus)}</p>
                  <p className="list-row-meta mt-2">{describeDecodeStatus(detailQuery.data.decodeStatus)}</p>
                </div>
              </div>
              <DecodeStatusLegend compact />
              <div className="workspace-block">
                <div className="workspace-section-label">Raw Payload</div>
                <pre className="field-shell w-full overflow-x-auto whitespace-pre-wrap text-xs leading-6">{detailQuery.data.payloadRaw}</pre>
              </div>
              <div className="workspace-block">
                <div className="workspace-section-label">Decoded Payload</div>
                <pre className="field-shell w-full overflow-x-auto whitespace-pre-wrap text-xs leading-6">{detailQuery.data.payloadDecoded ?? decodedPayloadFallback(detailQuery.data.decodeStatus)}</pre>
              </div>
              <div className="workspace-block">
                <div className="workspace-section-label">Headers</div>
                {detailQuery.data.headers.length ? (
                  <div className="list-stack">
                    {detailQuery.data.headers.map((header) => (
                      <div key={`${header.key}:${header.value}`} className="list-row">
                        <div>
                          <p className="list-row-title font-mono">{header.key}</p>
                          <p className="list-row-meta break-all">{header.value || '空'}</p>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <EmptyState title="当前没有 Header" description="这条消息没有携带可展示的消息头。" />
                )}
              </div>
            </div>
          ) : null}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">上下文</div>
          {detailQuery.data ? (
            <div className="list-stack">
              <div className="list-row">
                <div>
                  <p className="list-row-title">时间</p>
                  <p className="list-row-meta">{detailQuery.data.timestamp}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">Key</p>
                  <p className="list-row-meta font-mono">{detailQuery.data.keyRaw ?? '空'}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">Schema</p>
                  <p className="list-row-meta">{detailQuery.data.schemaInfo ?? '暂无'}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">运行时提示</p>
                  <p className="list-row-meta">{detailQuery.data.relatedHints?.join(' / ') || '当前没有附加提示'}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">书签状态</p>
                  <p className="list-row-meta">{activeBookmark ? `已收藏 · ${activeBookmark.createdAt}` : '未收藏'}</p>
                </div>
                <Badge tone={activeBookmark ? 'success' : 'muted'}>{activeBookmark ? '已收藏' : '未收藏'}</Badge>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">后续操作</p>
                  <p className="list-row-meta">可以从这条消息继续进入受控回放，或带着当前键值进入有界追踪。</p>
                </div>
              </div>
              <div className="workspace-actions">
                <Link
                  to={`/replay?topic=${encodeURIComponent(detailQuery.data.messageRef.topic)}&partition=${detailQuery.data.messageRef.partition}&offset=${encodeURIComponent(detailQuery.data.messageRef.offset)}`}
                  className="button-shell"
                  data-variant="primary"
                >
                  进入回放
                </Link>
                <button
                  type="button"
                  className="button-shell"
                  data-variant={activeBookmark ? 'ghost' : 'primary'}
                  disabled={isBookmarkBusy}
                  onClick={() => {
                    if (activeBookmark) {
                      deleteBookmarkMutation.mutate(activeBookmark.id);
                      return;
                    }

                    createBookmarkMutation.mutate({
                      messageRef: detailQuery.data.messageRef,
                      label: detailQuery.data.keyRaw?.trim() ? `Key ${detailQuery.data.keyRaw}` : undefined,
                    });
                  }}
                >
                  {isBookmarkBusy ? '处理中…' : activeBookmark ? '取消收藏' : '收藏消息'}
                </button>
                {traceLaunch ? (
                  <Link to={traceLaunch.href} className="button-shell" data-variant="ghost">
                    {traceLaunch.label}
                  </Link>
                ) : null}
              </div>
            </div>
          ) : (
            <EmptyState title="暂无上下文" description="加载消息详情后，这里会显示关键上下文。" />
          )}
        </aside>
      </section>
    </PageFrame>
  );
}
