import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Link, useSearchParams } from 'react-router-dom';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { EmptyState } from '@/components/ui/empty-state';
import { listMessageBookmarks } from '@/features/bookmarks/api';
import type { MessageBookmark } from '@/features/bookmarks/types';
import { createSavedQuery } from '@/features/saved-queries/api';
import { listSavedQueries } from '@/features/saved-queries/api';
import type { CreateSavedQueryInput, SavedQuery } from '@/features/saved-queries/types';
import { TableShell } from '@/components/ui/table-shell';
import { queryMessages } from '@/features/messages/api';
import type { HeaderFilter, MessageSummary } from '@/features/messages/types';
import { getAppPreferences } from '@/features/preferences/api';
import type { AppPreferences } from '@/features/preferences/types';
import { listTopics } from '@/features/topics/api';
import type { TopicSummary } from '@/features/topics/types';
import type { AppError } from '@/lib/tauri';
import { DecodeStatusLegend } from '@/components/messages/decode-status-legend';
import { formatDecodeStatus } from '@/features/messages/decode-status';

const MAX_RESULTS_CAP = 500;

function parsePartitions(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return [];
  }

  return trimmed
    .split(',')
    .map((item) => Number(item.trim()))
    .filter((item) => Number.isInteger(item) && item >= 0);
}

function formatDateTimeLocal(value: Date) {
  const year = value.getFullYear();
  const month = `${value.getMonth() + 1}`.padStart(2, '0');
  const day = `${value.getDate()}`.padStart(2, '0');
  const hours = `${value.getHours()}`.padStart(2, '0');
  const minutes = `${value.getMinutes()}`.padStart(2, '0');

  return `${year}-${month}-${day}T${hours}:${minutes}`;
}

function buildDefaultTimeRange(windowMinutes: number) {
  const end = new Date();
  const start = new Date(end.getTime() - windowMinutes * 60 * 1000);

  return {
    startTime: formatDateTimeLocal(start),
    endTime: formatDateTimeLocal(end),
  };
}

export function MessagesPage() {
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);
  const [topic, setTopic] = useState('');
  const [partitionsInput, setPartitionsInput] = useState('');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [startOffset, setStartOffset] = useState('');
  const [endOffset, setEndOffset] = useState('');
  const [keyFilter, setKeyFilter] = useState('');
  const [headerFilter, setHeaderFilter] = useState('');
  const [maxResults, setMaxResults] = useState(100);
  const [results, setResults] = useState<MessageSummary[]>([]);
  const [hasExecutedQuery, setHasExecutedQuery] = useState(false);
  const [saveQueryName, setSaveQueryName] = useState('');
  const [saveQueryDescription, setSaveQueryDescription] = useState('');
  const [lastAutoRunToken, setLastAutoRunToken] = useState('');

  const preferencesQuery = useQuery<AppPreferences, AppError>({
    queryKey: ['app-preferences'],
    queryFn: getAppPreferences,
  });

  const topicsQuery = useQuery<TopicSummary[], AppError>({
    queryKey: ['messages-topics', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listTopics({ clusterProfileId: activeClusterProfileId!, limit: 500 }),
  });

  const bookmarksQuery = useQuery<MessageBookmark[], AppError>({
    queryKey: ['message-bookmarks', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listMessageBookmarks({ clusterProfileId: activeClusterProfileId! }),
  });

  const savedQueriesQuery = useQuery<SavedQuery[], AppError>({
    queryKey: ['saved-queries'],
    queryFn: listSavedQueries,
  });

  useEffect(() => {
    if (!preferencesQuery.data) {
      return;
    }

    if (startTime || endTime) {
      return;
    }

    const defaults = buildDefaultTimeRange(preferencesQuery.data.defaultMessageQueryWindowMinutes);
    setStartTime(defaults.startTime);
    setEndTime(defaults.endTime);
  }, [endTime, preferencesQuery.data, startTime]);

  useEffect(() => {
    const topicParam = searchParams.get('topic');
    if (topicParam) setTopic(topicParam);
    const partitionsParam = searchParams.get('partitions');
    if (partitionsParam) setPartitionsInput(partitionsParam);
    const startTimeParam = searchParams.get('startTime');
    if (startTimeParam) setStartTime(startTimeParam);
    const endTimeParam = searchParams.get('endTime');
    if (endTimeParam) setEndTime(endTimeParam);
    const startOffsetParam = searchParams.get('startOffset');
    if (startOffsetParam) setStartOffset(startOffsetParam);
    const endOffsetParam = searchParams.get('endOffset');
    if (endOffsetParam) setEndOffset(endOffsetParam);
    const keyFilterParam = searchParams.get('keyFilter');
    if (keyFilterParam) setKeyFilter(keyFilterParam);
    const headerFilterParam = searchParams.get('headerFilter');
    if (headerFilterParam) setHeaderFilter(headerFilterParam);
    const maxResultsParam = searchParams.get('maxResults');
    if (maxResultsParam) setMaxResults(Number(maxResultsParam));
    const nameParam = searchParams.get('name');
    if (nameParam) setSaveQueryName(nameParam);
  }, [searchParams]);

  const partitions = useMemo(() => parsePartitions(partitionsInput), [partitionsInput]);
  const hasTimeBound = Boolean(startTime || endTime);
  const hasOffsetBound = Boolean(startOffset || endOffset);
  const isBounded = hasTimeBound || hasOffsetBound || partitions.length > 0;
  const validationMessage = useMemo(() => {
    if (!activeClusterProfileId) {
      return '请选择一个活动集群。';
    }

    if (!topic.trim()) {
      return '请选择一个主题。';
    }

    if (!isBounded) {
      return '查询必须设置边界条件：请至少提供分区、时间范围或偏移范围之一。';
    }

    if (maxResults <= 0 || maxResults > MAX_RESULTS_CAP) {
      return `最大结果数必须在 1 到 ${MAX_RESULTS_CAP} 之间。`;
    }

    return null;
  }, [activeClusterProfileId, isBounded, maxResults, topic]);

  const queryMutation = useMutation<MessageSummary[], AppError, void>({
    mutationFn: async () => {
      const headerFilters: HeaderFilter[] = headerFilter.trim()
        ? [
            (() => {
              const [key, value] = headerFilter.split('=', 2);
              return { key: key.trim(), value: value?.trim() || undefined };
            })(),
          ]
        : [];

      return queryMessages({
        clusterProfileId: activeClusterProfileId!,
        topic,
        partitions: partitions.length ? partitions : undefined,
        timeRange: hasTimeBound
          ? {
              start: startTime,
              end: endTime,
            }
          : undefined,
        offsetRange: hasOffsetBound
          ? {
              startOffset: startOffset || undefined,
              endOffset: endOffset || undefined,
            }
          : undefined,
        keyFilter: keyFilter.trim() || undefined,
        headerFilters: headerFilters.length ? headerFilters : undefined,
        maxResults,
      });
    },
    onSuccess: (data) => {
      setHasExecutedQuery(true);
      setResults(data);
    },
    onError: () => {
      setHasExecutedQuery(true);
      setResults([]);
    },
  });

  const saveQueryMutation = useMutation({
    mutationFn: createSavedQuery,
  });

  const currentHeaderFilters: HeaderFilter[] = headerFilter.trim()
    ? [
        (() => {
          const [key, value] = headerFilter.split('=', 2);
          return { key: key.trim(), value: value?.trim() || undefined };
        })(),
      ]
    : [];

  const canSaveCurrentQuery = Boolean(activeClusterProfileId && topic.trim() && saveQueryName.trim() && isBounded);

  useEffect(() => {
    const autoRun = searchParams.get('autoRun');
    const token = searchParams.toString();
    if (autoRun !== '1' || !activeClusterProfileId || validationMessage || !topic.trim()) {
      return;
    }
    if (lastAutoRunToken === token || queryMutation.isPending) {
      return;
    }

    setLastAutoRunToken(token);
    setHasExecutedQuery(false);
    queryMutation.mutate();
  }, [activeClusterProfileId, lastAutoRunToken, queryMutation, searchParams, topic, validationMessage]);

  return (
    <PageFrame
      eyebrow="消息排查"
      title="消息"
      description="先定义清晰边界，再执行消息查询。禁止整主题无边界扫描。"
      contextualInfo={<div className="workspace-note">当前工作区会沿用全局集群上下文，仅保留本页查询边界与重置动作。</div>}
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          <div className="workspace-toolbar" role="toolbar">
            <div className="workspace-actions">
              <button
                type="button"
                className="button-shell"
                data-variant="ghost"
                onClick={() => {
                  const defaults = buildDefaultTimeRange(
                    preferencesQuery.data?.defaultMessageQueryWindowMinutes ?? 30,
                  );
                  setPartitionsInput('');
                  setStartTime(defaults.startTime);
                  setEndTime(defaults.endTime);
                  setStartOffset('');
                  setEndOffset('');
                  setKeyFilter('');
                  setHeaderFilter('');
                  setMaxResults(100);
                  setHasExecutedQuery(false);
                  setResults([]);
                }}
              >
                重置条件
              </button>
            </div>
          </div>

          {!activeClusterProfileId ? (
            <EmptyState title="请先选择活动集群" description="消息查询依赖当前集群配置。" action={<Link to="/settings/cluster-profiles" className="button-shell" data-variant="primary">前往集群配置</Link>}  />
          ) : (
            <>
              {topicsQuery.isLoading ? (
                <div className="workspace-note py-6">正在读取当前集群的主题列表…</div>
              ) : topicsQuery.isError ? (
                <EmptyState
                  title="主题列表加载失败"
                  description={topicsQuery.error.message}
                  action={
                    <button type="button" className="button-shell" data-variant="primary" onClick={() => topicsQuery.refetch()}>
                      重试
                    </button>
                  }
                />
              ) : (
                <>
              <div className="toolbar-shell mb-3">
                <div className="lg:col-span-6">
                  <label className="field-label" htmlFor="messages-topic">主题</label>
                  <select id="messages-topic" className="field-shell w-full enhanced-select" value={topic} onChange={(event) => setTopic(event.target.value)}>
                    <option value="">请选择主题</option>
                    {(topicsQuery.data ?? []).map((item) => (
                      <option key={item.name} value={item.name}>
                        {item.name}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="lg:col-span-3">
                  <label className="field-label" htmlFor="messages-partitions">分区</label>
                  <input id="messages-partitions" className="field-shell w-full" value={partitionsInput} onChange={(event) => setPartitionsInput(event.target.value)} placeholder="请输入分区号，例如 0,1,2" />
                </div>
                <div className="lg:col-span-3">
                  <label className="field-label" htmlFor="messages-max-results">最大结果数</label>
                  <input
                    id="messages-max-results"
                    className="field-shell w-full"
                    type="number"
                    min={1}
                    max={MAX_RESULTS_CAP}
                    value={maxResults}
                    onChange={(event) => setMaxResults(Number(event.target.value))}
                  />
                </div>
              </div>

              <div className="form-grid mb-3">
                <div className="form-section">
                  <div className="form-section-title">时间范围</div>
                  <div className="form-section-description">至少填开始或结束时间之一。</div>
                  <div className="form-grid mt-3">
                    <div>
                      <label className="field-label" htmlFor="messages-start-time">开始时间</label>
                      <input id="messages-start-time" className="field-shell w-full" type="datetime-local" value={startTime} onChange={(event) => setStartTime(event.target.value)} />
                    </div>
                    <div>
                      <label className="field-label" htmlFor="messages-end-time">结束时间</label>
                      <input id="messages-end-time" className="field-shell w-full" type="datetime-local" value={endTime} onChange={(event) => setEndTime(event.target.value)} />
                    </div>
                  </div>
                </div>

                <div className="form-section">
                  <div className="form-section-title">偏移范围</div>
                  <div className="form-section-description">可替代时间范围，也可和分区一起使用。</div>
                  <div className="form-grid mt-3">
                    <div>
                      <label className="field-label" htmlFor="messages-start-offset">起始偏移</label>
                      <input id="messages-start-offset" className="field-shell w-full" value={startOffset} onChange={(event) => setStartOffset(event.target.value)} placeholder="请输入偏移值，例如 1200" />
                    </div>
                    <div>
                      <label className="field-label" htmlFor="messages-end-offset">结束偏移</label>
                      <input id="messages-end-offset" className="field-shell w-full" value={endOffset} onChange={(event) => setEndOffset(event.target.value)} placeholder="例如 1350" />
                    </div>
                  </div>
                </div>
              </div>

              <div className="toolbar-shell mb-3">
                <div className="lg:col-span-6">
                  <label className="field-label" htmlFor="messages-key-filter">Key 过滤</label>
                  <input id="messages-key-filter" className="field-shell w-full" value={keyFilter} onChange={(event) => setKeyFilter(event.target.value)} placeholder="可选，按 key 包含匹配" />
                </div>
                <div className="lg:col-span-6">
                  <label className="field-label" htmlFor="messages-header-filter">Header 过滤</label>
                  <input id="messages-header-filter" className="field-shell w-full" value={headerFilter} onChange={(event) => setHeaderFilter(event.target.value)} placeholder="可选，格式示例 traceId=abc123" />
                </div>
              </div>

              {validationMessage ? (
                <div className="feedback-banner mb-3" data-tone="warning">
                  {validationMessage}
                </div>
              ) : (
                <div className="feedback-banner mb-3" data-tone="signal">
                  查询边界有效，可以执行真实 Kafka 消息读取。
                </div>
              )}

              <div className="workspace-actions mb-3">
                <button
                  type="button"
                  className="button-shell"
                  data-variant="primary"
                  disabled={Boolean(validationMessage) || queryMutation.isPending}
                  onClick={() => {
                    setHasExecutedQuery(false);
                    queryMutation.mutate();
                  }}
                >
                  {queryMutation.isPending ? '读取中…' : '读取消息'}
                </button>
                <button
                  type="button"
                  className="button-shell"
                  data-variant="ghost"
                  disabled={!canSaveCurrentQuery || saveQueryMutation.isPending}
                  onClick={() => {
                    if (!activeClusterProfileId) return;
                    const request: CreateSavedQueryInput = {
                      name: saveQueryName.trim(),
                      queryType: 'messages',
                      clusterProfileId: activeClusterProfileId,
                      scopeJson: JSON.stringify({
                        topic,
                        partitions: partitions.length ? partitions : undefined,
                        timeRange: hasTimeBound ? { start: startTime, end: endTime } : undefined,
                        offsetRange: hasOffsetBound
                          ? { startOffset: startOffset || undefined, endOffset: endOffset || undefined }
                          : undefined,
                      }),
                      queryJson: JSON.stringify({
                        keyFilter: keyFilter.trim() || undefined,
                        headerFilters: currentHeaderFilters.length ? currentHeaderFilters : undefined,
                        maxResults,
                      }),
                      description: saveQueryDescription.trim() || undefined,
                      isFavorite: false,
                    };
                    saveQueryMutation.mutate(request, {
                      onSuccess: async () => {
                        setSaveQueryDescription('');
                        await queryClient.invalidateQueries({ queryKey: ['saved-queries'] });
                      },
                    });
                  }}
                >
                  {saveQueryMutation.isPending ? '保存中…' : '保存查询'}
                </button>
              </div>

              <div className="toolbar-shell mb-3">
                <div className="lg:col-span-4">
                  <label className="field-label" htmlFor="messages-save-query-name">查询名称</label>
                  <input id="messages-save-query-name" className="field-shell w-full" value={saveQueryName} onChange={(event) => setSaveQueryName(event.target.value)} placeholder="例如 Orders 近 30 分钟排查" />
                </div>
                <div className="lg:col-span-8">
                  <label className="field-label" htmlFor="messages-save-query-description">查询说明</label>
                  <input id="messages-save-query-description" className="field-shell w-full" value={saveQueryDescription} onChange={(event) => setSaveQueryDescription(event.target.value)} placeholder="可选，记录适用环境、排查目标或注意事项" />
                </div>
              </div>

              {saveQueryMutation.isError ? (
                <div className="feedback-banner mb-3" data-tone="danger">
                  {saveQueryMutation.error.message}
                </div>
              ) : saveQueryMutation.isSuccess ? (
                <div className="feedback-banner mb-3" data-tone="success">
                  当前查询已保存，可在“保存的查询”页面继续打开、编辑或删除。
                </div>
              ) : null}

              {queryMutation.isPending ? (
                <div className="workspace-note py-6">正在读取 Kafka 消息…</div>
              ) : queryMutation.isError ? (
                <EmptyState title="消息读取失败" description={queryMutation.error.message} />
              ) : hasExecutedQuery ? (
                <div className="list-stack">
                  <DecodeStatusLegend compact />
                  <TableShell
                    initialVisibleRowCount={50}
                    rowLabel="条消息"
                    columns={['时间', '分区', '偏移', 'Key', '解码', 'Payload 预览', '操作']}
                    emptyState={<EmptyState title="没有查询结果" description="当前边界内没有命中消息。" />}
                  >
                    {results.map((item) => (
                      <tr key={`${item.messageRef.topic}-${item.messageRef.partition}-${item.messageRef.offset}`}>
                        <td className="font-mono text-xs text-ink-dim">{item.timestamp}</td>
                        <td>{item.partition}</td>
                        <td className="font-mono text-xs text-ink-dim">{item.offset}</td>
                        <td>{item.keyPreview ?? '—'}</td>
                        <td>{formatDecodeStatus(item.decodeStatus)}</td>
                        <td>{item.payloadPreview ?? '—'}</td>
                        <td>
                          <Link
                            to={`/messages/${encodeURIComponent(item.messageRef.topic)}/${item.messageRef.partition}/${encodeURIComponent(item.messageRef.offset)}`}
                            className="button-shell"
                            data-variant="ghost"
                          >
                            详情
                          </Link>
                        </td>
                      </tr>
                    ))}
                  </TableShell>
                </div>
              ) : (
                <EmptyState
                  title="查询结果区"
                  description={
                    validationMessage
                      ? '先把查询范围限定好，再执行消息读取。'
                      : `已准备执行：主题 ${topic}，分区 ${partitions.length ? partitions.join(', ') : '自动'}，最多 ${maxResults} 条。`
                  }
                />
              )}
                </>
              )}
            </>
          )}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">查询规则</div>
          <div className="list-stack">
            <div className="list-row">
              <div>
                <p className="list-row-title">必须有边界</p>
                <p className="list-row-meta">至少要提供分区、时间范围或偏移范围之一。</p>
              </div>
            </div>
            <div className="list-row">
              <div>
                <p className="list-row-title">禁止全量扫描</p>
                <p className="list-row-meta">不允许“整主题无限制读取”。</p>
              </div>
            </div>
            <div className="list-row">
              <div>
                <p className="list-row-title">结果数受限</p>
                <p className="list-row-meta">默认 100，最大 {MAX_RESULTS_CAP}。</p>
              </div>
            </div>
            <div className="list-row">
              <div>
                <p className="list-row-title">默认时间窗口</p>
                <p className="list-row-meta">
                  {preferencesQuery.isLoading
                    ? '正在读取你的本地默认查询窗口。'
                    : preferencesQuery.isError
                      ? `默认窗口加载失败：${preferencesQuery.error.message}`
                      : preferencesQuery.data
                    ? `已按偏好预填最近 ${preferencesQuery.data.defaultMessageQueryWindowMinutes} 分钟。`
                    : '当前未读取到本地默认查询窗口。'}
                </p>
              </div>
            </div>
          </div>

          <div className="workspace-section-label mt-4">最近保存的查询</div>
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
                  <p className="list-row-meta">{query.lastRunAt ?? '未运行'}</p>
                </div>
                <Link to="/saved-queries" className="button-shell" data-variant="ghost">
                  打开
                </Link>
              </div>
            ))}
            {!savedQueriesQuery.isLoading && !savedQueriesQuery.isError && !savedQueriesQuery.data?.length ? <EmptyState title="暂无保存查询" description="保存常用消息排查条件后，这里会显示查询模板。" /> : null}
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
                  <p className="list-row-meta">{bookmark.messageRef.partition} / {bookmark.messageRef.offset}</p>
                </div>
                <Link to={`/messages/${encodeURIComponent(bookmark.messageRef.topic)}/${bookmark.messageRef.partition}/${encodeURIComponent(bookmark.messageRef.offset)}`} className="button-shell" data-variant="ghost">
                  查看
                </Link>
              </div>
            ))}
            {!bookmarksQuery.isLoading && !bookmarksQuery.isError && !bookmarksQuery.data?.length ? <EmptyState title="暂无收藏消息" description="在消息详情页收藏后，这里会显示最近入口。" /> : null}
          </div>
        </aside>
      </section>
    </PageFrame>
  );
}
