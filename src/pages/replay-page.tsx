import { useEffect, useMemo, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { EmptyState } from '@/components/ui/empty-state';
import { getMessageDetail } from '@/features/messages/api';
import { getReplayPolicy } from '@/features/replay-policy/api';
import type { ReplayPolicy } from '@/features/replay-policy/types';
import { createReplayJob, getReplayJob, listReplayJobs } from '@/features/replay/api';
import type { MessageDetailResponse, MessageHeader, MessageRef } from '@/features/messages/types';
import type { ReplayJobDetailResponse, ReplayJobEvent, ReplayJobSummary } from '@/features/replay/types';
import type { AppError } from '@/lib/tauri';
import { Badge } from '@/components/ui/badge';

type FeedbackState = {
  tone: 'success' | 'warning' | 'danger' | 'signal';
  message: string;
  detail?: string;
};

type ParsedReplayResultSummary = {
  mode?: string;
  executionStage?: string;
  deliveryConfirmed?: boolean;
  note?: string;
  error?: string;
  delivery?: {
    partition?: number;
    offset?: number;
    timestamp?: number | null;
  };
};

function parseStoredEditValue(input?: string | null, field?: 'key' | 'payload') {
  if (!input) {
    return '';
  }

  try {
    const parsed = JSON.parse(input) as { key?: string; payload?: string };
    if (field === 'key') {
      return parsed.key ?? '';
    }
    if (field === 'payload') {
      return parsed.payload ?? '';
    }
    return '';
  } catch {
    return '';
  }
}

function parseStoredHeaders(input?: string | null) {
  if (!input) {
    return '';
  }

  try {
    const parsed = JSON.parse(input) as MessageHeader[];
    return parsed.map((header) => `${header.key}=${header.value}`).join(', ');
  } catch {
    return '';
  }
}

function parseEditedHeaders(input: string): { headers: MessageHeader[] | null; error: string | null } {
  const normalized = input.trim();
  if (!normalized) {
    return { headers: null, error: null };
  }

  const segments = normalized
    .split(/\n|,/)
    .map((segment) => segment.trim())
    .filter(Boolean);

  const headers: MessageHeader[] = [];

  for (const segment of segments) {
    const separatorIndex = segment.indexOf('=');
    if (separatorIndex <= 0) {
      return {
        headers: null,
        error: 'Headers 必须使用 key=value 格式，可按逗号或换行分隔。',
      };
    }

    const key = segment.slice(0, separatorIndex).trim();
    const value = segment.slice(separatorIndex + 1).trim();

    if (!key) {
      return {
        headers: null,
        error: 'Header 键不能为空。',
      };
    }

    headers.push({ key, value });
  }

  return { headers, error: null };
}

function getReplayStatusLabel(status: string) {
  switch (status) {
    case 'accepted':
      return '已接受';
    case 'publishing':
      return '投递中';
    case 'delivered':
      return '已投递';
    case 'delivery_unknown':
      return '结果未知';
    case 'validated':
      return '已验证';
    case 'queued_local':
      return '本地排队';
    case 'succeeded':
      return '已完成';
    case 'queued':
      return '已排队';
    case 'failed':
      return '失败';
    default:
      return status;
  }
}

function summarizeEvent(event: ReplayJobEvent) {
  switch (event.eventType) {
    case 'validated':
      return '请求已通过基本校验。';
    case 'local_validation_completed':
      return 'Dry Run 已完成本地校验，本地记录已落库。';
    case 'dry_run_completed':
      return 'Dry Run 已完成，本地记录已落库。';
    case 'queued':
      return 'Live Replay 已入队，等待后续执行链路。';
    case 'queued_local':
      return '回放请求已在本地排队并写入审计，当前实现尚未执行 broker 投递。';
    case 'accepted':
      return '回放请求已接受，等待 broker 投递执行。';
    case 'publishing':
      return '正在等待 broker 投递确认。';
    case 'delivery_confirmed':
      return '已收到 broker 投递确认。';
    case 'delivery_failed':
      return 'broker 投递失败，任务已标记为失败。';
    case 'delivery_unknown_recovered':
      return '应用重启后检测到未完成投递，结果被标记为未知。';
    default:
      return event.eventType;
  }
}

function parseReplayResultSummary(input?: string | null): ParsedReplayResultSummary | null {
  if (!input) {
    return null;
  }

  try {
    return JSON.parse(input) as ParsedReplayResultSummary;
  } catch {
    return null;
  }
}

function getReplayModeLabel(mode: string) {
  return mode === 'dry-run' ? 'Dry Run' : mode === 'broker-delivery' ? 'Broker 投递' : '本地排队';
}

export function ReplayPage() {
  const [searchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);
  const activeClusterName = useWorkbenchStore((state) => state.activeClusterName);

  const sourceTopic = searchParams.get('topic') ?? '';
  const sourcePartition = searchParams.get('partition') ?? '';
  const sourceOffset = searchParams.get('offset') ?? '';

  const [targetTopic, setTargetTopic] = useState('');
  const [editedKey, setEditedKey] = useState('');
  const [editedPayload, setEditedPayload] = useState('');
  const [editedHeaders, setEditedHeaders] = useState('');
  const [dryRun, setDryRun] = useState(true);
  const [riskAcknowledged, setRiskAcknowledged] = useState(false);
  const [feedback, setFeedback] = useState<FeedbackState | null>(null);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);

  const sourceDetailQuery = useQuery<MessageDetailResponse, AppError>({
    queryKey: ['replay-source', activeClusterProfileId, sourceTopic, sourcePartition, sourceOffset],
    enabled: Boolean(activeClusterProfileId && sourceTopic && sourcePartition && sourceOffset),
    queryFn: () =>
      getMessageDetail({
        clusterProfileId: activeClusterProfileId!,
        topic: sourceTopic,
        partition: Number(sourcePartition),
        offset: sourceOffset,
      }),
  });

  const replayJobsQuery = useQuery<ReplayJobSummary[], AppError>({
    queryKey: ['replay-jobs', activeClusterProfileId],
    enabled: Boolean(activeClusterProfileId),
    queryFn: () => listReplayJobs(activeClusterProfileId!),
  });

  const replayPolicyQuery = useQuery<ReplayPolicy, AppError>({
    queryKey: ['replay-policy'],
    queryFn: getReplayPolicy,
  });

  const replayJobDetailQuery = useQuery<ReplayJobDetailResponse, AppError>({
    queryKey: ['replay-job', selectedJobId],
    enabled: Boolean(selectedJobId),
    queryFn: () => getReplayJob(selectedJobId!),
  });

  useEffect(() => {
    if (!replayJobsQuery.data?.length) {
      if (selectedJobId) {
        setSelectedJobId(null);
      }
      return;
    }

    const hasSelectedJob = selectedJobId
      ? replayJobsQuery.data.some((job) => job.id === selectedJobId)
      : false;

    if (!hasSelectedJob) {
      setSelectedJobId(replayJobsQuery.data[0].id);
    }
  }, [replayJobsQuery.data, selectedJobId]);

  const parsedHeaders = useMemo(() => parseEditedHeaders(editedHeaders), [editedHeaders]);
  const selectedReplaySummary = useMemo(
    () => parseReplayResultSummary(replayJobDetailQuery.data?.job.resultSummaryJson),
    [replayJobDetailQuery.data?.job.resultSummaryJson],
  );

  const validationMessage = useMemo(() => {
    if (!activeClusterProfileId) {
      return '请选择一个活动集群。';
    }
    if (!sourceTopic || !sourcePartition || !sourceOffset) {
      return '必须从具体消息发起回放。';
    }
    if (sourceDetailQuery.isLoading) {
      return '正在校验来源消息。';
    }
    if (sourceDetailQuery.isError || !sourceDetailQuery.data) {
      return '来源消息尚未成功加载，暂时不能提交回放。';
    }
    if (parsedHeaders.error) {
      return parsedHeaders.error;
    }
    if (!dryRun && replayPolicyQuery.isLoading) {
      return '正在加载回放策略，暂时不能提交 Broker 投递回放。';
    }
    if (!dryRun && (replayPolicyQuery.isError || !replayPolicyQuery.data)) {
      return '回放策略尚未成功加载，暂时不能提交 Broker 投递回放。';
    }
    if (!dryRun && replayPolicyQuery.data && !replayPolicyQuery.data.allowLiveReplay) {
      return '当前策略禁止提交 Broker 投递回放。';
    }
    if (!targetTopic.trim()) {
      return '必须明确填写目标主题。';
    }
    if (
      !dryRun &&
      replayPolicyQuery.data?.sandboxOnly &&
      !targetTopic.trim().startsWith(replayPolicyQuery.data.sandboxTopicPrefix)
    ) {
      return `当前策略只允许将 Broker 投递回放提交到前缀为 ${replayPolicyQuery.data.sandboxTopicPrefix} 的主题。`;
    }
    if (!dryRun && replayPolicyQuery.data?.requireRiskAcknowledgement && !riskAcknowledged) {
      return '当前策略要求先完成风险确认。';
    }
    if (!dryRun && !riskAcknowledged) {
      return '提交 Broker 投递回放前必须明确确认风险。';
    }
    return null;
  }, [
    activeClusterProfileId,
    dryRun,
    parsedHeaders.error,
    replayPolicyQuery.isError,
    replayPolicyQuery.isLoading,
    replayPolicyQuery.data,
    riskAcknowledged,
    sourceDetailQuery.data,
    sourceDetailQuery.isError,
    sourceDetailQuery.isLoading,
    sourceOffset,
    sourcePartition,
    sourceTopic,
    targetTopic,
  ]);

  const createMutation = useMutation({
    mutationFn: createReplayJob,
    onSuccess: async (response: ReplayJobDetailResponse) => {
      setFeedback({
        tone:
          response.job.status === 'validated' || response.job.status === 'delivered'
            ? 'success'
            : response.job.status === 'failed'
              ? 'danger'
              : 'warning',
        message:
          response.job.status === 'validated'
            ? 'Dry Run 已完成本地校验并写入记录。'
            : response.job.status === 'delivered'
              ? 'Replay 已收到 broker 投递确认。'
              : response.job.status === 'failed'
                ? 'Replay broker 投递失败，已写入本地审计。'
                : 'Replay 请求已提交，正在等待执行结果。',
        detail: response.auditRef ? `审计引用：${response.auditRef}` : undefined,
      });
      setSelectedJobId(response.job.id);
      queryClient.setQueryData(['replay-job', response.job.id], response);
      await queryClient.invalidateQueries({ queryKey: ['replay-jobs', activeClusterProfileId] });
    },
    onError: (error: AppError) => {
      setFeedback({
        tone: 'danger',
        message: error.message,
        detail: `错误代码：${error.code}`,
      });
    },
  });

  const handleSubmit = () => {
    setFeedback(null);

    if (validationMessage || !activeClusterProfileId) {
      return;
    }

    const sourceMessageRef: MessageRef = {
      clusterProfileId: activeClusterProfileId,
      topic: sourceTopic,
      partition: Number(sourcePartition),
      offset: sourceOffset,
    };

    createMutation.mutate({
      clusterProfileId: activeClusterProfileId,
      sourceMessageRef,
      sourceTimestamp: sourceDetailQuery.data?.timestamp ?? null,
      targetTopic: targetTopic.trim(),
      editedKey: editedKey.trim() || null,
      editedHeaders: parsedHeaders.headers,
      editedPayload: editedPayload.trim() || null,
      dryRun,
      riskAcknowledged,
    });
  };

  const handleRemediateReplay = () => {
    const job = replayJobDetailQuery.data?.job;
    if (!job) {
      return;
    }

    setTargetTopic(job.targetTopic);
    setEditedKey(parseStoredEditValue(job.keyEditJson, 'key'));
    setEditedPayload(parseStoredEditValue(job.payloadEditJson, 'payload'));
    setEditedHeaders(parseStoredHeaders(job.headersEditJson));
    setDryRun(false);
    setRiskAcknowledged(false);
    setFeedback({
      tone: 'signal',
      message: '已将上一次回放任务复制回表单。',
      detail: '请复核目标主题、编辑内容和当前集群能力，并重新完成风险确认后提交。',
    });
  };

  return (
    <PageFrame
      eyebrow="受控回放"
      title="回放"
      description="只能从明确来源消息发起，必须显式指定目标并完成风险确认。当前版本提供 Dry Run 与受控 Broker 投递。"
      contextualInfo={
        <div>
          <div className="workspace-title">回放向导</div>
          <div className="workspace-note">
            {activeClusterProfileId ? `当前按 ${activeClusterName} 执行受控回放` : '未选择集群'}
          </div>
        </div>
      }
      actions={
        <div className="page-actions">
          <button
            type="button"
            className="button-shell"
            data-variant="primary"
            disabled={Boolean(validationMessage) || createMutation.isPending}
            onClick={handleSubmit}
          >
            {createMutation.isPending ? '正在提交…' : dryRun ? '执行 Dry Run' : '提交 Broker 投递回放'}
          </button>
          <Link to="/messages" className="button-shell" data-variant="ghost">
            返回消息查询
          </Link>
        </div>
      }
    >
      <section className="workspace-surface">
        <div className="workspace-main" role="main">
          {!activeClusterProfileId ? (
            <EmptyState title="请先选择活动集群" description="回放依赖当前集群配置。"  />
          ) : (
            <>
                <div className="workspace-block" aria-labelledby="replay-source-section-title">
                  <h2 id="replay-source-section-title" className="workspace-section-label">1. 来源确认</h2>
                  {!sourceTopic || !sourcePartition || !sourceOffset ? (
                    <EmptyState title="缺少来源消息" description="请先从消息详情页进入回放。" />
                  ) : sourceDetailQuery.isLoading ? (
                    <div className="workspace-note py-4" role="status" aria-live="polite">正在加载来源消息…</div>
                  ) : sourceDetailQuery.isError ? (
                    <EmptyState title="来源消息读取失败" description={sourceDetailQuery.error.message} />
                  ) : sourceDetailQuery.data ? (
                  <div className="list-stack">
                    <div className="list-row">
                      <div>
                        <p className="list-row-title">来源引用</p>
                        <p className="list-row-meta font-mono">
                          {sourceTopic} / {sourcePartition} / {sourceOffset}
                        </p>
                      </div>
                    </div>
                    <div className="list-row">
                      <div>
                        <p className="list-row-title">来源时间</p>
                        <p className="list-row-meta">{sourceDetailQuery.data.timestamp || '未记录'}</p>
                      </div>
                    </div>
                    <div className="list-row">
                      <div>
                        <p className="list-row-title">Payload 预览</p>
                        <p className="list-row-meta">{sourceDetailQuery.data.payloadRaw.slice(0, 180) || '空消息'}</p>
                      </div>
                    </div>
                  </div>
                ) : null}
              </div>

                <div className="workspace-block" aria-labelledby="replay-target-section-title">
                  <h2 id="replay-target-section-title" className="workspace-section-label">2. 目标选择</h2>
                  <label className="field-label" htmlFor="replay-target-topic">
                    目标主题
                  </label>
                <input
                  id="replay-target-topic"
                  className="field-shell w-full"
                    value={targetTopic}
                    onChange={(event) => setTargetTopic(event.target.value)}
                    placeholder="请输入目标主题名称"
                    aria-describedby="replay-target-topic-hint"
                  />
                  <p id="replay-target-topic-hint" className="mt-2 text-xs text-ink-muted">
                    回放不会沿用默认目标，必须由操作者显式输入主题名称。
                  </p>
                </div>

                <div className="workspace-block" aria-labelledby="replay-payload-section-title">
                  <h2 id="replay-payload-section-title" className="workspace-section-label">3. 内容调整</h2>
                  <div className="form-grid">
                    <div>
                      <label className="field-label" htmlFor="replay-edited-key">
                      编辑 Key
                    </label>
                    <input
                      id="replay-edited-key"
                      className="field-shell w-full"
                      value={editedKey}
                      onChange={(event) => setEditedKey(event.target.value)}
                      placeholder="可选项，留空则使用原值"
                    />
                  </div>
                  <div>
                    <label className="field-label" htmlFor="replay-edited-headers">
                      编辑 Headers
                    </label>
                      <input
                        id="replay-edited-headers"
                        className="field-shell w-full"
                      value={editedHeaders}
                      onChange={(event) => setEditedHeaders(event.target.value)}
                        placeholder="可选项，格式示例 traceId=abc123, tenant=prod"
                        aria-describedby={parsedHeaders.error ? 'replay-edited-headers-error' : 'replay-edited-headers-hint'}
                      />
                      <p id="replay-edited-headers-hint" className="mt-2 text-xs text-ink-muted">
                        使用 key=value 格式，可用逗号或换行分隔多个 Header。
                      </p>
                      {parsedHeaders.error ? (
                        <p id="replay-edited-headers-error" className="mt-2 text-xs text-red-500" >
                          {parsedHeaders.error}
                        </p>
                      ) : null}
                    </div>
                  </div>
                  <label className="field-label mt-3" htmlFor="replay-edited-payload">
                  编辑 Payload
                </label>
                <textarea
                  id="replay-edited-payload"
                  className="field-shell min-h-40 w-full"
                  value={editedPayload}
                  onChange={(event) => setEditedPayload(event.target.value)}
                    placeholder="可选项，留空则保留原始 payload"
                  />
                </div>

                <div className="workspace-block" aria-labelledby="replay-risk-section-title">
                  <h2 id="replay-risk-section-title" className="workspace-section-label">4. 风险确认</h2>
                  <div className="list-stack">
                    <div className="list-row">
                      <div>
                      <p className="list-row-title">执行模式</p>
                      <p className="list-row-meta">
                        {dryRun
                          ? '当前为 Dry Run，仅做本地校验与记录，不执行 broker 写入。'
                          : '当前为 Broker 投递模式：将尝试执行真实 broker 写入，并记录投递结果。'}
                      </p>
                    </div>
                      <button
                        type="button"
                        className="button-shell"
                        data-variant="ghost"
                        aria-pressed={!dryRun}
                        aria-label={dryRun ? '切换到 Broker 投递模式' : '切换到 Dry Run 模式'}
                        disabled={!replayPolicyQuery.data?.allowLiveReplay && dryRun}
                        onClick={() => setDryRun((current) => !current)}
                      >
                      {dryRun ? '切到 Broker 投递' : '切到 Dry Run'}
                    </button>
                  </div>
                  <div className="list-row">
                    <div>
                      <p className="list-row-title">风险确认</p>
                      <p className="list-row-meta">Broker 投递模式必须明确确认风险，并留下本地审计记录。</p>
                    </div>
                      <button
                        type="button"
                        className="button-shell"
                        data-variant={riskAcknowledged ? 'primary' : 'ghost'}
                        aria-pressed={riskAcknowledged}
                        aria-label={riskAcknowledged ? '取消风险确认' : '确认风险'}
                        onClick={() => setRiskAcknowledged((current) => !current)}
                      >
                        {riskAcknowledged ? '已确认风险' : '确认风险'}
                    </button>
                  </div>
                </div>
              </div>

              {validationMessage ? (
                <div className="feedback-banner mt-3" data-tone="warning" role="status" aria-live="polite">
                  {validationMessage}
                </div>
              ) : (
                <div className="feedback-banner mt-3" data-tone="signal" role="status" aria-live="polite">
                  条件已完整，可以提交回放请求并查看审计与事件历史。
                </div>
              )}

              {feedback ? (
                <div className="feedback-banner mt-3" data-tone={feedback.tone} role={feedback.tone === 'danger' ? 'alert' : 'status'} aria-live="polite">
                  <div>{feedback.message}</div>
                  {feedback.detail ? <div className="workspace-note mt-1">{feedback.detail}</div> : null}
                </div>
              ) : null}

              <div className="workspace-block" aria-labelledby="replay-result-section-title">
                <h2 id="replay-result-section-title" className="workspace-section-label">5. 执行结果</h2>
                {!selectedJobId ? (
                  <EmptyState
                    title="尚未创建回放任务"
                    description="提交 Dry Run 或 Broker 投递请求后，这里会显示执行记录与事件历史。"
                  />
                ) : replayJobDetailQuery.isLoading ? (
                  <div className="workspace-note py-4" role="status" aria-live="polite">正在加载回放结果…</div>
                ) : replayJobDetailQuery.isError ? (
                  <EmptyState
                    title="回放结果读取失败"
                    description={replayJobDetailQuery.error.message}
                    action={
                      <button
                        type="button"
                        className="button-shell"
                        data-variant="primary"
                        onClick={() => replayJobDetailQuery.refetch()}
                      >
                        重试
                      </button>
                    }
                  />
                ) : replayJobDetailQuery.data ? (
                  <div className="list-stack">
                    {replayJobDetailQuery.data.job.status === 'delivery_unknown' ? (
                      <div className="feedback-banner" data-tone="warning" role="status" aria-live="polite">
                        <div>上一次 broker 投递在应用中断后未能确认结果。</div>
                        <div className="workspace-note mt-1">
                          建议先检查目标系统，再将该任务复制回表单重新执行。
                        </div>
                        <div className="workspace-actions mt-3">
                          <button
                            type="button"
                            className="button-shell"
                            data-variant="primary"
                            onClick={handleRemediateReplay}
                          >
                            复制回表单重试
                          </button>
                        </div>
                      </div>
                    ) : null}

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">任务状态</p>
                        <p className="list-row-meta">{getReplayStatusLabel(replayJobDetailQuery.data.job.status)}</p>
                      </div>
                      <Badge
                        tone={
                          replayJobDetailQuery.data.job.status === 'validated' ||
                          replayJobDetailQuery.data.job.status === 'delivered'
                            ? 'success'
                            : replayJobDetailQuery.data.job.status === 'failed'
                              ? 'danger'
                              : 'warning'
                        }
                      >
                        {replayJobDetailQuery.data.job.riskLevel}
                      </Badge>
                    </div>

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">执行模式</p>
                        <p className="list-row-meta">{getReplayModeLabel(replayJobDetailQuery.data.job.mode)}</p>
                      </div>
                    </div>

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">目标主题</p>
                        <p className="list-row-meta font-mono">{replayJobDetailQuery.data.job.targetTopic}</p>
                      </div>
                    </div>

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">来源引用</p>
                        <p className="list-row-meta font-mono">{`${replayJobDetailQuery.data.job.sourceTopic} / ${replayJobDetailQuery.data.job.sourcePartition} / ${replayJobDetailQuery.data.job.sourceOffset}`}</p>
                      </div>
                    </div>

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">来源时间</p>
                        <p className="list-row-meta">{replayJobDetailQuery.data.job.sourceTimestamp ?? '未记录'}</p>
                      </div>
                    </div>

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">创建 / 完成时间</p>
                        <p className="list-row-meta">
                          {replayJobDetailQuery.data.job.createdAt} → {replayJobDetailQuery.data.job.completedAt ?? '未完成'}
                        </p>
                      </div>
                    </div>

                    {replayJobDetailQuery.data.job.errorMessage ? (
                      <div className="list-row">
                        <div>
                          <p className="list-row-title">错误信息</p>
                          <p className="list-row-meta">{replayJobDetailQuery.data.job.errorMessage}</p>
                        </div>
                      </div>
                    ) : null}

                    {selectedReplaySummary ? (
                      <div className="list-row">
                        <div>
                          <p className="list-row-title">执行摘要</p>
                          <p className="list-row-meta">
                            {selectedReplaySummary.note ?? '无摘要'}
                            {selectedReplaySummary.executionStage
                              ? ` · 阶段 ${selectedReplaySummary.executionStage}`
                              : ''}
                            {typeof selectedReplaySummary.deliveryConfirmed === 'boolean'
                              ? ` · Broker 投递 ${selectedReplaySummary.deliveryConfirmed ? '已确认' : '未确认'}`
                              : ''}
                          </p>
                          {selectedReplaySummary.delivery ? (
                            <p className="mt-1 text-[0.72rem] text-ink-muted">
                              partition={selectedReplaySummary.delivery.partition ?? '-'} · offset=
                              {selectedReplaySummary.delivery.offset ?? '-'}
                              {selectedReplaySummary.delivery.timestamp != null
                                ? ` · ts=${selectedReplaySummary.delivery.timestamp}`
                                : ''}
                            </p>
                          ) : null}
                          {selectedReplaySummary.error ? (
                            <p className="mt-1 text-[0.72rem] text-red-500">{selectedReplaySummary.error}</p>
                          ) : null}
                        </div>
                      </div>
                    ) : null}

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">编辑覆盖</p>
                        <p className="list-row-meta">
                          {[
                            replayJobDetailQuery.data.job.keyEditJson ? 'Key 覆盖' : null,
                            replayJobDetailQuery.data.job.headersEditJson ? 'Headers 覆盖' : null,
                            replayJobDetailQuery.data.job.payloadEditJson ? 'Payload 覆盖' : null,
                          ]
                            .filter(Boolean)
                            .join(' / ') || '无覆盖，使用来源消息原始内容'}
                        </p>
                      </div>
                    </div>

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">审计引用</p>
                        <p className="list-row-meta font-mono">
                          {replayJobDetailQuery.data.auditRef ?? '本次结果未返回审计引用'}
                        </p>
                      </div>
                    </div>

                    <div className="list-row">
                      <div>
                        <p className="list-row-title">事件历史</p>
                        <p className="list-row-meta">共 {replayJobDetailQuery.data.eventHistory.length} 条</p>
                      </div>
                    </div>

                    {replayJobDetailQuery.data.eventHistory.map((event) => (
                      <div key={event.id} className="list-row">
                        <div>
                          <p className="list-row-title">{summarizeEvent(event)}</p>
                          <p className="list-row-meta">{event.createdAt}</p>
                          {event.eventPayloadJson ? (
                            <pre className="field-shell mt-2 w-full overflow-x-auto whitespace-pre-wrap text-xs leading-6">
                              {event.eventPayloadJson}
                            </pre>
                          ) : null}
                        </div>
                      </div>
                    ))}
                  </div>
                ) : null}
              </div>
            </>
          )}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">固定摘要</div>
          <div className="list-stack">
            <div className="list-row">
              <div>
                <p className="list-row-title">来源</p>
                <p className="list-row-meta font-mono">
                  {sourceTopic && sourcePartition && sourceOffset
                    ? `${sourceTopic} / ${sourcePartition} / ${sourceOffset}`
                    : '未选择'}
                </p>
                <p className="list-row-meta">{sourceDetailQuery.data?.timestamp ?? '未记录来源时间'}</p>
              </div>
            </div>

            <div className="list-row">
              <div>
                <p className="list-row-title">目标</p>
                <p className="list-row-meta">{targetTopic || '未填写'}</p>
              </div>
            </div>

            <div className="list-row">
              <div>
                <p className="list-row-title">修改项</p>
                <p className="list-row-meta">
                  {[editedKey && 'Key 已改', editedHeaders && 'Headers 已改', editedPayload && 'Payload 已改']
                    .filter(Boolean)
                    .join(' / ') || '无修改'}
                </p>
              </div>
            </div>

            <div className="list-row">
              <div>
                <p className="list-row-title">风险级别</p>
                <p className="list-row-meta">{dryRun ? '低（Dry Run）' : '高（Broker 投递）'}</p>
              </div>
            </div>

            <div className="list-row">
              <div>
                <p className="list-row-title">策略边界</p>
                <p className="list-row-meta">
                  {replayPolicyQuery.data
                    ? replayPolicyQuery.data.sandboxOnly
                      ? `Broker 投递仅允许 ${replayPolicyQuery.data.sandboxTopicPrefix}*`
                      : '当前不限制目标主题前缀'
                    : '正在读取运行时回放策略'}
                </p>
                {replayPolicyQuery.data ? (
                  <p className="list-row-meta mt-1">
                    超时 {replayPolicyQuery.data.deliveryTimeoutSeconds}s / 最多重试{' '}
                    {replayPolicyQuery.data.maxRetryAttempts} 次
                  </p>
                ) : null}
              </div>
            </div>
          </div>

          <div className="workspace-section-label mt-4">最近回放</div>
          {!activeClusterProfileId ? (
            <EmptyState title="未选择集群" description="选择活动集群后，这里会显示最近的回放任务。" />
          ) : replayJobsQuery.isLoading ? (
            <div className="workspace-note py-4" role="status" aria-live="polite">正在加载回放历史…</div>
          ) : replayJobsQuery.isError ? (
            <EmptyState title="回放历史加载失败" description={replayJobsQuery.error.message} />
          ) : replayJobsQuery.data?.length ? (
            <div className="list-stack mt-3">
              {replayJobsQuery.data.map((job) => (
                <button
                  key={job.id}
                  type="button"
                  className="button-shell w-full justify-between"
                  data-variant={selectedJobId === job.id ? 'primary' : 'ghost'}
                  aria-pressed={selectedJobId === job.id}
                  aria-label={`查看 ${job.targetTopic} 的${job.mode === 'dry-run' ? ' Dry Run' : ' Broker 投递'}回放结果，当前状态 ${getReplayStatusLabel(job.status)}`}
                  onClick={() => setSelectedJobId(job.id)}
                >
                  <span className="truncate">
                    {job.mode === 'dry-run' ? 'Dry Run' : 'Broker 投递'} · {job.targetTopic}
                  </span>
                  <span className="text-xs text-current/70">{getReplayStatusLabel(job.status)}</span>
                </button>
              ))}
            </div>
          ) : (
            <EmptyState title="暂无回放历史" description="先执行一次 Dry Run 或 Broker 投递请求，这里会保留最近任务。" />
          )}
        </aside>
      </section>
    </PageFrame>
  );
}
