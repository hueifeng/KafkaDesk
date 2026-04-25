import { useEffect, useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { useMutation, useQuery } from '@tanstack/react-query';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';
import { listCorrelationRules } from '@/features/correlation/api';
import type { CorrelationRule } from '@/features/correlation/types';
import { getAppPreferences } from '@/features/preferences/api';
import type { AppPreferences } from '@/features/preferences/types';
import { runTraceQuery } from '@/features/trace/api';
import type { RunTraceQueryInput, TraceEvent, TraceQueryResult, TraceResultMode } from '@/features/trace/types';
import type { AppError } from '@/lib/tauri';

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

function parseTopicScope(input: string) {
  return input
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}

function buildKeyTypeOptions(rules: CorrelationRule[]) {
  const options = [{ value: 'message-key', label: '消息 Key' }];
  const headerKeys = new Set<string>();

  for (const rule of rules) {
    if (!rule.isEnabled || rule.matchStrategy !== 'header-match') {
      continue;
    }

    try {
      const parsed = JSON.parse(rule.ruleJson) as { matchKey?: string };
      if (parsed.matchKey?.trim()) {
        headerKeys.add(parsed.matchKey.trim());
      }
    } catch {
      continue;
    }
  }

  return [
    ...options,
    ...Array.from(headerKeys).map((headerKey) => ({ value: `header:${headerKey}`, label: `Header / ${headerKey}` })),
  ];
}

function buildSelectedKeyTypeOption(keyType: string) {
  if (keyType === 'message-key') {
    return { value: 'message-key', label: '消息 Key' };
  }

  const headerKey = keyType.startsWith('header:') ? keyType.slice('header:'.length).trim() : '';
  if (!headerKey) {
    return null;
  }

  return {
    value: `header:${headerKey}`,
    label: `Header / ${headerKey}（当前未配置规则）`,
  };
}

function renderTraceNotes(notes?: string[] | null) {
  return notes?.filter(Boolean) ?? [];
}

export function TracePage() {
  const [searchParams] = useSearchParams();
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);

  const [keyType, setKeyType] = useState(searchParams.get('keyType') ?? 'message-key');
  const [keyValue, setKeyValue] = useState(searchParams.get('keyValue') ?? '');
  const [topicScopeInput, setTopicScopeInput] = useState(searchParams.get('topicScope') ?? '');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [resultMode, setResultMode] = useState<TraceResultMode>('timeline');
  const [hasExecutedTrace, setHasExecutedTrace] = useState(false);

  const preferencesQuery = useQuery<AppPreferences, AppError>({
    queryKey: ['app-preferences'],
    queryFn: getAppPreferences,
  });

  const correlationRulesQuery = useQuery<CorrelationRule[], AppError>({
    queryKey: ['correlation-rules'],
    queryFn: listCorrelationRules,
  });

  useEffect(() => {
    if (!preferencesQuery.data) {
      return;
    }

    setResultMode(preferencesQuery.data.preferredTraceView);

    if (!startTime && !endTime) {
      const defaults = buildDefaultTimeRange(preferencesQuery.data.defaultMessageQueryWindowMinutes);
      setStartTime(defaults.startTime);
      setEndTime(defaults.endTime);
    }
  }, [endTime, preferencesQuery.data, startTime]);

  const enabledRules = useMemo(
    () => (correlationRulesQuery.data ?? []).filter((rule) => rule.isEnabled && rule.clusterProfileId === activeClusterProfileId),
    [activeClusterProfileId, correlationRulesQuery.data],
  );
  const keyTypeOptions = useMemo(() => {
    const options = buildKeyTypeOptions(enabledRules);
    if (options.some((option) => option.value === keyType)) {
      return options;
    }

    const selectedOption = buildSelectedKeyTypeOption(keyType);
    return selectedOption ? [...options, selectedOption] : options;
  }, [enabledRules, keyType]);
  const topicScope = useMemo(() => parseTopicScope(topicScopeInput), [topicScopeInput]);
  const selectedKeyTypeHasRule = useMemo(
    () => keyType === 'message-key' || enabledRules.some((rule) => keyType.startsWith('header:') && (() => {
      if (rule.matchStrategy !== 'header-match') {
        return false;
      }

      try {
        const parsed = JSON.parse(rule.ruleJson) as { matchKey?: string };
        return `header:${parsed.matchKey?.trim() ?? ''}` === keyType;
      } catch {
        return false;
      }
    })()),
    [enabledRules, keyType],
  );

  useEffect(() => {
    if (!keyTypeOptions.some((option) => option.value === keyType)) {
      setKeyType(keyTypeOptions[0]?.value ?? 'message-key');
    }
  }, [keyType, keyTypeOptions]);

  const validationMessage = useMemo(() => {
    if (!activeClusterProfileId) {
      return '请选择一个活动集群。';
    }
    if (!keyType.trim()) {
      return 'You must select a trace key type.';
    }
    if (!keyValue.trim()) {
      return '请填写追踪键值。';
    }
    if (!startTime || !endTime) {
      return '请指定时间范围。';
    }
    if (!topicScope.length) {
      return '请至少指定一个主题范围以避免无边界追踪。';
    }
    return null;
  }, [activeClusterProfileId, endTime, keyType, keyValue, startTime, topicScope.length]);

  const traceMutation = useMutation<TraceQueryResult, AppError, RunTraceQueryInput>({
    mutationFn: runTraceQuery,
    onMutate: () => {
      setHasExecutedTrace(false);
    },
    onSuccess: () => {
      setHasExecutedTrace(true);
    },
    onError: () => {
      setHasExecutedTrace(true);
    },
  });

  const traceResult = traceMutation.data;
  const activeEvents: TraceEvent[] = resultMode === 'table' ? traceResult?.events ?? [] : traceResult?.timeline ?? [];

  return (
    <PageFrame
      eyebrow="事件追踪"
      title="追踪"
      description="按键值在时间与主题范围内追踪事件。"
      contextualInfo={<div><div className="workspace-note">聚焦追踪条件与结果模式。</div></div>}
      actions={
        <button
          type="button"
          className="button-shell"
          data-variant="primary"
          disabled={Boolean(validationMessage) || traceMutation.isPending}
          onClick={() =>
            traceMutation.mutate({
              clusterProfileId: activeClusterProfileId!,
              keyType,
              keyValue: keyValue.trim(),
              topicScope,
              timeRange: { start: startTime, end: endTime },
              resultMode,
            })
          }
        >
          {traceMutation.isPending ? '追踪中…' : '执行追踪'}
        </button>
      }
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          {!activeClusterProfileId ? (
            <EmptyState title="请先选择活动集群" description="追踪依赖当前集群配置。" />
          ) : correlationRulesQuery.isLoading ? (
            <div className="workspace-note py-6">正在读取关联规则…</div>
          ) : correlationRulesQuery.isError ? (
            <EmptyState
              title="追踪规则加载失败"
              description={correlationRulesQuery.error.message}
              action={
                <button type="button" className="button-shell" data-variant="primary" onClick={() => correlationRulesQuery.refetch()}>
                  重试
                </button>
              }
            />
          ) : (
            <>
              <div className="toolbar-shell mb-3">
                <div className="lg:col-span-3">
                  <label className="field-label" htmlFor="trace-key-type">键类型</label>
                  <select id="trace-key-type" className="field-shell w-full" value={keyType} onChange={(event) => setKeyType(event.target.value)}>
                    {keyTypeOptions.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="lg:col-span-3">
                  <label className="field-label" htmlFor="trace-key-value">键值</label>
                  <input id="trace-key-value" className="field-shell w-full" value={keyValue} onChange={(event) => setKeyValue(event.target.value)} placeholder="请输入 traceId、orderId 或消息 key" />
                </div>
                <div className="lg:col-span-6">
                  <label className="field-label" htmlFor="trace-topic-scope">主题范围</label>
                  <input id="trace-topic-scope" className="field-shell w-full" value={topicScopeInput} onChange={(event) => setTopicScopeInput(event.target.value)} placeholder="以逗号分隔主题，例如 orders.events,payments.events" />
                </div>
              </div>

              <div className="form-grid mb-3">
                <div>
                  <label className="field-label" htmlFor="trace-start-time">开始时间</label>
                  <input id="trace-start-time" className="field-shell w-full" type="datetime-local" value={startTime} onChange={(event) => setStartTime(event.target.value)} />
                </div>
                <div>
                  <label className="field-label" htmlFor="trace-end-time">结束时间</label>
                  <input id="trace-end-time" className="field-shell w-full" type="datetime-local" value={endTime} onChange={(event) => setEndTime(event.target.value)} />
                </div>
                <div>
                  <label className="field-label" htmlFor="trace-result-mode">结果模式</label>
                  <select id="trace-result-mode" className="field-shell w-full" value={resultMode} onChange={(event) => setResultMode(event.target.value as TraceResultMode)}>
                    <option value="timeline">时间线</option>
                    <option value="table">表格</option>
                  </select>
                </div>
              </div>

              {validationMessage ? (
                <div className="feedback-banner mb-3" data-tone="warning">
                  {validationMessage}
                </div>
              ) : (
                <div className="feedback-banner mb-3" data-tone="signal">
                  查询已保持有界，可按主题与时间范围追踪。
                </div>
              )}

              {!selectedKeyTypeHasRule && keyType.startsWith('header:') ? (
                <div className="feedback-banner mb-3" data-tone="warning">
                  当前 Header 键没有匹配的已启用关联规则；本次追踪只会使用你手动提供的主题范围，不会自动补全跨主题链路。
                </div>
              ) : null}

              {preferencesQuery.isError ? (
                <div className="feedback-banner mb-3" data-tone="warning">
                  本地偏好读取失败：{preferencesQuery.error.message}。请手动确认时间范围与结果模式。
                </div>
              ) : null}

              {traceMutation.isPending ? (
                <div className="workspace-note py-6">正在按当前边界执行追踪…</div>
              ) : traceMutation.isError ? (
                <EmptyState title="追踪加载失败" description={traceMutation.error.message} />
              ) : traceResult ? (
                <>
                  <div className="workspace-block">
                    <div className="workspace-section-label">结果摘要</div>
                    <div className="list-stack">
                      <div className="list-row">
                        <div>
                          <p className="list-row-title">扫描范围</p>
                          <p className="list-row-meta">{traceResult.querySummary.scannedTopics.join(', ')}</p>
                        </div>
                      </div>
                      <div className="list-row">
                        <div>
                          <p className="list-row-title">命中数量</p>
                          <p className="list-row-meta">{traceResult.querySummary.matchedCount}</p>
                        </div>
                      </div>
                    </div>
                  </div>

                  <TableShell
                    initialVisibleRowCount={50}
                    rowLabel="条事件"
                    columns={['时间', '主题', '分区', '偏移', '匹配方式', 'Key', 'Payload 预览', '操作']}
                    emptyState={<EmptyState title="当前没有命中事件" description="请调整键值、主题范围或时间窗口。" />}
                  >
                    {activeEvents.map((event) => (
                      <tr key={`${event.messageRef.topic}-${event.messageRef.partition}-${event.messageRef.offset}`}>
                        <td className="font-mono text-xs text-ink-dim">{event.timestamp}</td>
                        <td>{event.topic}</td>
                        <td>{event.partition}</td>
                        <td className="font-mono text-xs text-ink-dim">{event.offset}</td>
                        <td>{event.matchedBy}</td>
                        <td>{event.keyPreview ?? '—'}</td>
                        <td>{event.payloadPreview ?? '—'}</td>
                        <td>
                          <Link to={`/messages/${encodeURIComponent(event.messageRef.topic)}/${event.messageRef.partition}/${encodeURIComponent(event.messageRef.offset)}`} className="button-shell" data-variant="ghost">
                            消息详情
                          </Link>
                        </td>
                      </tr>
                    ))}
                  </TableShell>
                </>
              ) : hasExecutedTrace ? (
                <EmptyState title="暂无命中事件" description="调整键值、主题范围或时间窗口后重试。" />
              ) : (
                <EmptyState title="暂无追踪结果" description="执行追踪后，这里会展示 timeline 或 table 结果，并支持打开消息详情。" />
              )}
            </>
          )}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">追踪说明</div>
          <div className="list-stack">
            {correlationRulesQuery.isSuccess ? (
              <div className="list-row">
                <div>
                  <p className="list-row-title">关联规则</p>
                  <p className="list-row-meta">
                    {enabledRules.length
                      ? `当前集群已启用 ${enabledRules.length} 条关联规则。`
                      : '当前集群没有已启用的关联规则；仍可按消息 Key 执行有界追踪。'}
                  </p>
                </div>
              </div>
            ) : null}
            {!selectedKeyTypeHasRule && keyType.startsWith('header:') ? (
              <div className="list-row">
                <div>
                  <p className="list-row-title">缺失关联规则</p>
                  <p className="list-row-meta">当前键类型未命中已启用规则；如果需要自动扩展主题范围，请先补齐对应 Header 关联规则。</p>
                </div>
              </div>
            ) : null}
            {renderTraceNotes(traceResult?.confidenceNotes).map((note) => (
              <div key={note} className="list-row">
                <div>
                  <p className="list-row-title">说明</p>
                  <p className="list-row-meta">{note}</p>
                </div>
              </div>
            ))}
            {!traceResult?.confidenceNotes?.length ? (
              <div className="list-row">
                <div>
                  <p className="list-row-title">当前边界</p>
                  <p className="list-row-meta">只做有界同步追踪；图谱、缓存、异步任务后续再补。</p>
                </div>
              </div>
            ) : null}
            {preferencesQuery.isLoading ? (
              <div className="list-row">
                <div>
                  <p className="list-row-title">本地偏好</p>
                  <p className="list-row-meta">正在读取默认时间窗口与结果模式。</p>
                </div>
              </div>
            ) : null}
          </div>
        </aside>
      </section>
    </PageFrame>
  );
}
