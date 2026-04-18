import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { useWorkbenchStore } from '@/app/workbench-store';
import { PageFrame } from '@/components/layout/page-frame';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';
import { listClusters } from '@/features/clusters/api';
import { mapClusterToStorePayload } from '@/features/clusters/shared-helpers';
import type { ClusterProfileSummary } from '@/features/clusters/types';
import { deleteSavedQuery, listSavedQueries, updateSavedQuery } from '@/features/saved-queries/api';
import {
  parseSavedMessagesQuery,
  parseSavedMessagesScope,
  type SavedQuery,
  type UpdateSavedQueryInput,
} from '@/features/saved-queries/types';
import type { AppError } from '@/lib/tauri';

function formatSavedQueryType(queryType: SavedQuery['queryType']) {
  return queryType === 'messages' ? '消息查询' : queryType;
}

function toSearchParams(query: SavedQuery) {
  const scope = parseSavedMessagesScope(query);
  const filters = parseSavedMessagesQuery(query);
  if (!scope || !filters) {
    return '';
  }

  const params = new URLSearchParams();
  params.set('topic', scope.topic);
  if (scope.partitions?.length) {
    params.set('partitions', scope.partitions.join(','));
  }
  if (scope.timeRange?.start) {
    params.set('startTime', scope.timeRange.start);
  }
  if (scope.timeRange?.end) {
    params.set('endTime', scope.timeRange.end);
  }
  if (scope.offsetRange?.startOffset) {
    params.set('startOffset', scope.offsetRange.startOffset);
  }
  if (scope.offsetRange?.endOffset) {
    params.set('endOffset', scope.offsetRange.endOffset);
  }
  if (filters.keyFilter) {
    params.set('keyFilter', filters.keyFilter);
  }
  if (filters.headerFilters?.[0]?.key) {
    params.set('headerFilter', `${filters.headerFilters[0].key}=${filters.headerFilters[0].value ?? ''}`);
  }
  params.set('clusterProfileId', query.clusterProfileId);
  params.set('name', query.name);
  params.set('autoRun', '1');
  params.set('maxResults', String(filters.maxResults));
  return params.toString();
}

function toUpdateInput(query: SavedQuery, updates?: Partial<UpdateSavedQueryInput>): UpdateSavedQueryInput {
  return {
    id: query.id,
    name: updates?.name ?? query.name,
    queryType: updates?.queryType ?? query.queryType,
    clusterProfileId: updates?.clusterProfileId ?? query.clusterProfileId,
    scopeJson: updates?.scopeJson ?? query.scopeJson,
    queryJson: updates?.queryJson ?? query.queryJson,
    description: updates?.description ?? query.description ?? null,
    isFavorite: updates?.isFavorite ?? query.isFavorite,
    lastRunAt: updates?.lastRunAt ?? query.lastRunAt ?? null,
  };
}

export function SavedQueriesPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const setClusterContext = useWorkbenchStore((state) => state.setClusterContext);
  const [selectedQueryId, setSelectedQueryId] = useState<string | null>(null);
  const [formState, setFormState] = useState<UpdateSavedQueryInput | null>(null);
  const [feedback, setFeedback] = useState<{ tone: 'success' | 'warning' | 'danger'; message: string } | null>(null);

  const savedQueriesQuery = useQuery<SavedQuery[], AppError>({
    queryKey: ['saved-queries'],
    queryFn: listSavedQueries,
  });

  const clustersQuery = useQuery<ClusterProfileSummary[], AppError>({
    queryKey: ['clusters'],
    queryFn: listClusters,
  });

  useEffect(() => {
    const first = savedQueriesQuery.data?.[0] ?? null;
    if (!first) {
      setSelectedQueryId(null);
      setFormState(null);
      return;
    }

    const active = selectedQueryId ? savedQueriesQuery.data?.find((item) => item.id === selectedQueryId) ?? first : first;
    if (selectedQueryId !== active.id) {
      setSelectedQueryId(active.id);
      return;
    }

    setFormState(toUpdateInput(active));
  }, [savedQueriesQuery.data, selectedQueryId]);

  const updateMutation = useMutation({
    mutationFn: updateSavedQuery,
    onSuccess: async (query: SavedQuery) => {
      setFeedback({ tone: 'success', message: `消息查询“${query.name}”已保存。` });
      await queryClient.invalidateQueries({ queryKey: ['saved-queries'] });
    },
    onError: (error: AppError) => setFeedback({ tone: 'danger', message: error.message }),
  });

  const deleteMutation = useMutation({
    mutationFn: deleteSavedQuery,
    onSuccess: async () => {
      setFeedback({ tone: 'success', message: '已删除保存的消息查询。' });
      setSelectedQueryId(null);
      await queryClient.invalidateQueries({ queryKey: ['saved-queries'] });
    },
    onError: (error: AppError) => setFeedback({ tone: 'danger', message: error.message }),
  });

  return (
    <PageFrame
      eyebrow="复用消息查询"
      title="保存的消息查询"
      description="把常用的消息查询保存下来，减少重复输入。当前页面只支持消息查询。"
      contextualInfo={<div className="workspace-note">这里只管理消息查询的保存项，不包含 topics、groups、trace 等其他调查类型。</div>}
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          {savedQueriesQuery.isLoading ? (
            <div className="workspace-note py-6">正在加载已保存的消息查询…</div>
          ) : savedQueriesQuery.isError ? (
            <EmptyState title="保存的消息查询加载失败" description="请检查连接状态后重试。" />
          ) : (
            <>
              <TableShell
                columns={['查询名称', '查询类型', '集群', '最近运行', '收藏', '操作']}
                emptyState={<EmptyState title="还没有保存的消息查询" description="先在消息页保存一个常用查询，后续就能直接复用。" />}
              >
                {(savedQueriesQuery.data ?? []).map((item) => {
                  const cluster = clustersQuery.data?.find((entry) => entry.id === item.clusterProfileId) ?? null;
                  const clusterName = cluster?.name ?? item.clusterProfileId;

                  return (
                    <tr key={item.id}>
                      <td className="font-medium text-ink">{item.name}</td>
                      <td>{formatSavedQueryType(item.queryType)}</td>
                      <td>{clusterName}</td>
                      <td>{item.lastRunAt ?? '未运行'}</td>
                      <td>{item.isFavorite ? '★' : '—'}</td>
                      <td>
                        <div className="flex gap-2">
                          <button
                            type="button"
                            className="button-shell"
                            data-variant={selectedQueryId === item.id ? 'primary' : 'ghost'}
                            onClick={() => {
                              setSelectedQueryId(item.id);
                              setFeedback(null);
                            }}
                          >
                            选择
                          </button>
                          <button
                            type="button"
                            className="button-shell"
                            data-variant="ghost"
                            onClick={() => {
                              const lastRunAt = new Date().toISOString();
                              const openedQuery = { ...item, lastRunAt };
                              const params = toSearchParams(openedQuery);
                              if (!params) {
                                setFeedback({ tone: 'danger', message: `消息查询“${item.name}”的保存内容已损坏，无法恢复到消息页，请检查后重试。` });
                                return;
                              }

                              updateMutation.mutate(toUpdateInput(item, { lastRunAt }), {
                                onSuccess: () => {
                                  if (cluster) {
                                    setClusterContext(mapClusterToStorePayload(cluster));
                                  }

                                  navigate(`/messages?${params}`);
                                },
                              });
                            }}
                          >
                            打开
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </TableShell>

              {formState ? (
                <div className="workspace-block mt-4">
                  <div className="form-grid">
                    <div>
                      <label className="field-label" htmlFor="saved-query-name">查询名称</label>
                      <input id="saved-query-name" className="field-shell w-full" value={formState.name} onChange={(event) => setFormState((current) => (current ? { ...current, name: event.target.value } : current))} />
                    </div>
                    <div>
                      <label className="field-label" htmlFor="saved-query-cluster">集群</label>
                      <select id="saved-query-cluster" className="field-shell w-full" value={formState.clusterProfileId} onChange={(event) => setFormState((current) => (current ? { ...current, clusterProfileId: event.target.value } : current))}>
                        {(clustersQuery.data ?? []).map((cluster) => (
                          <option key={cluster.id} value={cluster.id}>
                            {cluster.name}
                          </option>
                        ))}
                      </select>
                    </div>
                    <div>
                      <label className="field-label" htmlFor="saved-query-type">查询类型</label>
                      <input id="saved-query-type" className="field-shell w-full" value={formatSavedQueryType(formState.queryType)} disabled />
                    </div>
                    <div>
                      <span className="field-label">收藏</span>
                      <button type="button" className="button-shell w-full justify-center" data-variant={formState.isFavorite ? 'primary' : 'ghost'} onClick={() => setFormState((current) => (current ? { ...current, isFavorite: !current.isFavorite } : current))}>
                        {formState.isFavorite ? '已收藏' : '未收藏'}
                      </button>
                    </div>
                  </div>
                  <div className="mt-3">
                    <label className="field-label" htmlFor="saved-query-description">说明</label>
                    <input id="saved-query-description" className="field-shell w-full" value={formState.description ?? ''} onChange={(event) => setFormState((current) => (current ? { ...current, description: event.target.value } : current))} />
                  </div>
                  <div className="form-grid mt-3">
                    <div>
                      <label className="field-label" htmlFor="saved-query-scope-json">查询范围 JSON</label>
                      <textarea id="saved-query-scope-json" className="field-shell min-h-40 w-full font-mono text-xs leading-6" value={formState.scopeJson} onChange={(event) => setFormState((current) => (current ? { ...current, scopeJson: event.target.value } : current))} />
                    </div>
                    <div>
                      <label className="field-label" htmlFor="saved-query-query-json">查询条件 JSON</label>
                      <textarea id="saved-query-query-json" className="field-shell min-h-40 w-full font-mono text-xs leading-6" value={formState.queryJson} onChange={(event) => setFormState((current) => (current ? { ...current, queryJson: event.target.value } : current))} />
                    </div>
                  </div>
                  <div className="workspace-actions mt-3">
                    <button type="button" className="button-shell" data-variant="primary" disabled={updateMutation.isPending} onClick={() => formState && updateMutation.mutate(formState)}>
                      {updateMutation.isPending ? '保存中…' : '保存修改'}
                    </button>
                    <button type="button" className="button-shell" data-variant="ghost" disabled={deleteMutation.isPending} onClick={() => deleteMutation.mutate(formState.id)}>
                      {deleteMutation.isPending ? '删除中…' : '删除查询'}
                    </button>
                  </div>
                </div>
              ) : null}

              {feedback ? (
                <div className="feedback-banner mt-3" data-tone={feedback.tone}>
                  {feedback.message}
                </div>
              ) : null}
            </>
          )}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">当前支持范围</div>
          <div className="list-stack">
            <div className="list-row">
              <div>
                <p className="list-row-title">当前仅支持消息查询</p>
                <p className="list-row-meta">先把高频、边界清晰的消息排查沉淀成可复用查询，topics、groups、trace 等类型后续再补。</p>
              </div>
            </div>
          </div>
        </aside>
      </section>
    </PageFrame>
  );
}
