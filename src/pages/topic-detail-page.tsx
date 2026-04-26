import { useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { PageFrame } from '@/components/layout/page-frame';
import { useWorkbenchStore } from '@/app/workbench-store';
import { expandTopicPartitions, getTopicDetail, getTopicOperationsOverview, updateTopicConfig } from '@/features/topics/api';
import type {
  ExpandTopicPartitionsInput,
  ExpandTopicPartitionsResponse,
  TopicDetailResponse,
  TopicOperationConfigEntry,
  TopicOperationsOverviewResponse,
  UpdateTopicConfigInput,
  UpdateTopicConfigResponse,
} from '@/features/topics/types';
import type { AppError, ValidationStageStatus } from '@/lib/tauri';
import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';

function toBadgeTone(status: ValidationStageStatus): 'success' | 'warning' | 'danger' | 'muted' | 'signal' {
  switch (status) {
    case 'passed':
      return 'success';
    case 'warning':
      return 'warning';
    case 'failed':
      return 'danger';
    case 'skipped':
      return 'muted';
    default:
      return 'signal';
  }
}

function formatStageStatus(status: ValidationStageStatus) {
  switch (status) {
    case 'passed':
      return '已通过';
    case 'warning':
      return '有提示';
    case 'failed':
      return '失败';
    case 'skipped':
      return '未开放';
    default:
      return status;
  }
}

function formatConfigValue(entry: TopicOperationConfigEntry) {
  if (!entry.isSupported) {
    return '当前未返回';
  }

  if (entry.value === null || entry.value === undefined || entry.value === '') {
    return '已返回，但当前没有具体值';
  }

  return entry.value;
}

function normalizeConfigEntryValue(value?: string | null) {
  return value ?? '';
}

function normalizeRequestedValueForSubmit(configKey: string, value: string) {
  return isNumericTopicConfigKey(configKey) ? value.trim() : value;
}

function describeConfigMetadata(entry: TopicOperationConfigEntry) {
  if (!entry.isSupported) {
    return '该配置项在当前集群中未返回，元数据保持未知。';
  }

  const details = [entry.source ? `来源：${entry.source}` : null];

  if (entry.isReadOnly === true) {
    details.push('只读');
  } else if (entry.isReadOnly === false) {
    details.push('可变更');
  }

  if (entry.isDefault === true) {
    details.push('默认值');
  } else if (entry.isDefault === false) {
    details.push('已覆盖默认值');
  }

  if (entry.isSensitive === true) {
    details.push('敏感配置');
  }

  return details.filter(Boolean).join(' · ') || 'Kafka 已返回该配置项。';
}

function isEditableConfigEntry(entry: TopicOperationConfigEntry) {
  return entry.isSupported && entry.isReadOnly === false;
}

function isNumericTopicConfigKey(configKey: string) {
  return configKey === 'retention.ms' || configKey === 'max.message.bytes';
}

function describeEditableConfigHint(configKey: string) {
  switch (configKey) {
    case 'cleanup.policy':
      return '使用逗号分隔的 Kafka 策略值，例如 compact、delete 或 compact,delete。';
    case 'retention.ms':
      return '请输入保留时长的毫秒值，例如 604800000 代表 7 天。';
    case 'max.message.bytes':
      return '请输入单条消息允许的最大字节数，通常用于限制超大消息写入。';
    default:
      return '请输入 broker 接受的新配置值。';
  }
}

const CLEANUP_POLICY_OPTIONS = ['compact', 'delete'] as const;

function serializeCleanupPolicy(values: string[]) {
  return CLEANUP_POLICY_OPTIONS.filter((option) => values.includes(option)).join(',');
}

function parseCleanupPolicy(value: string) {
  const parts = value
    .split(',')
    .map((part) => part.trim())
    .filter(Boolean);

  const unique = Array.from(new Set(parts));
  const hasUnknown = unique.some((part) => !CLEANUP_POLICY_OPTIONS.includes(part as (typeof CLEANUP_POLICY_OPTIONS)[number]));

  return {
    supported: !hasUnknown,
    values: CLEANUP_POLICY_OPTIONS.filter((option) => unique.includes(option)),
  };
}

function isDigitsOnly(value: string) {
  return /^\d+$/.test(value);
}

function formatDurationHint(value: string) {
  if (!isDigitsOnly(value)) {
    return null;
  }

  const milliseconds = Number(value);
  if (!Number.isFinite(milliseconds)) {
    return null;
  }

  const day = 24 * 60 * 60 * 1000;
  const hour = 60 * 60 * 1000;
  const minute = 60 * 1000;

  if (milliseconds >= day) {
    return `约 ${(milliseconds / day).toFixed(milliseconds % day === 0 ? 0 : 1)} 天`;
  }

  if (milliseconds >= hour) {
    return `约 ${(milliseconds / hour).toFixed(milliseconds % hour === 0 ? 0 : 1)} 小时`;
  }

  if (milliseconds >= minute) {
    return `约 ${(milliseconds / minute).toFixed(milliseconds % minute === 0 ? 0 : 1)} 分钟`;
  }

  return `约 ${milliseconds} ms`;
}

function formatBytesHint(value: string) {
  if (!isDigitsOnly(value)) {
    return null;
  }

  const bytes = Number(value);
  if (!Number.isFinite(bytes)) {
    return null;
  }

  if (bytes >= 1024 * 1024) {
    return `约 ${(bytes / (1024 * 1024)).toFixed(bytes % (1024 * 1024) === 0 ? 0 : 1)} MB`;
  }

  if (bytes >= 1024) {
    return `约 ${(bytes / 1024).toFixed(bytes % 1024 === 0 ? 0 : 1)} KB`;
  }

  return `约 ${bytes} bytes`;
}

function getNumericAdvisoryWarning(configKey: string, value: string) {
  if (!isDigitsOnly(value)) {
    return null;
  }

  const numericValue = Number(value);
  if (!Number.isFinite(numericValue)) {
    return null;
  }

  if (configKey === 'retention.ms' && (numericValue < 60_000 || numericValue > 31_536_000_000)) {
    return '这个 retention.ms 值明显偏短或偏长，提交前建议再次确认是否符合预期。';
  }

  if (configKey === 'max.message.bytes' && (numericValue < 1_024 || numericValue > 16_777_216)) {
    return '这个 max.message.bytes 值明显偏小或偏大，提交前建议再次确认是否符合预期。';
  }

  return null;
}



type TopicConfigDraft = {
  configKey: string;
  requestedValue: string;
  expectedCurrentValue: string;
  riskAcknowledged: boolean;
};

type TopicConfigFeedback = {
  tone: 'success' | 'warning' | 'danger';
  message: string;
  details?: string[];
};

type TopicConfigLastApplied = {
  configKey: string;
  previousValue?: string | null;
  resultingValue?: string | null;
  auditRef?: string | null;
  warning?: string | null;
};

type TopicPartitionExpansionDraft = {
  requestedPartitionCount: string;
  expectedCurrentPartitionCount: number;
  riskAcknowledged: boolean;
};

type TopicPartitionExpansionLastApplied = {
  previousPartitionCount: number;
  requestedPartitionCount: number;
  resultingPartitionCount: number;
  auditRef?: string | null;
  warning?: string | null;
};

export function TopicDetailPage() {
  const queryClient = useQueryClient();
  const { topicName } = useParams<{ topicName: string }>();
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);
  const decodedTopicName = topicName ? decodeURIComponent(topicName) : null;
  const [draft, setDraft] = useState<TopicConfigDraft | null>(null);
  const [partitionDraft, setPartitionDraft] = useState<TopicPartitionExpansionDraft | null>(null);
  const [feedback, setFeedback] = useState<TopicConfigFeedback | null>(null);
  const [lastApplied, setLastApplied] = useState<TopicConfigLastApplied | null>(null);
  const [lastPartitionExpansion, setLastPartitionExpansion] = useState<TopicPartitionExpansionLastApplied | null>(null);

  const detailQuery = useQuery<TopicDetailResponse, AppError>({
    queryKey: ['topic-detail', activeClusterProfileId, topicName],
    enabled: Boolean(activeClusterProfileId && decodedTopicName),
    queryFn: () => getTopicDetail(activeClusterProfileId!, decodedTopicName!),
  });

  const operationsOverviewQuery = useQuery<TopicOperationsOverviewResponse, AppError>({
    queryKey: ['topic-operations-overview', activeClusterProfileId, topicName],
    enabled: Boolean(activeClusterProfileId && decodedTopicName),
    queryFn: () => getTopicOperationsOverview(activeClusterProfileId!, decodedTopicName!),
  });

  const editableConfigEntries = useMemo(
    () => operationsOverviewQuery.data?.configEntries.filter(isEditableConfigEntry) ?? [],
    [operationsOverviewQuery.data?.configEntries],
  );

  const selectedConfigEntry = useMemo(
    () => operationsOverviewQuery.data?.configEntries.find((entry) => entry.key === draft?.configKey) ?? null,
    [draft?.configKey, operationsOverviewQuery.data?.configEntries],
  );

  const cleanupPolicyState = useMemo(
    () => (draft?.configKey === 'cleanup.policy' ? parseCleanupPolicy(draft.requestedValue) : null),
    [draft?.configKey, draft?.requestedValue],
  );

  const updateMutation = useMutation<UpdateTopicConfigResponse, AppError, UpdateTopicConfigInput>({
    mutationFn: updateTopicConfig,
    onSuccess: async (result) => {
      const details = [
        result.resultingValue ? `结果值：${result.resultingValue}` : null,
        result.auditRef ? `审计引用：${result.auditRef}` : '审计引用暂未写入，请查看后台日志。',
        result.warning ?? null,
      ].filter((item): item is string => Boolean(item));

      setFeedback({
        tone: result.warning ? 'warning' : 'success',
        message: `已应用 “${result.configKey}” 的配置修改。`,
        details,
      });
      setLastApplied({
        configKey: result.configKey,
        previousValue: result.previousValue,
        resultingValue: result.resultingValue,
        auditRef: result.auditRef,
        warning: result.warning,
      });
      setDraft(null);
      await queryClient.invalidateQueries({ queryKey: ['topic-detail', activeClusterProfileId, topicName] });
      await queryClient.invalidateQueries({ queryKey: ['topic-operations-overview', activeClusterProfileId, topicName] });
    },
    onError: (error) => {
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const partitionExpansionMutation = useMutation<ExpandTopicPartitionsResponse, AppError, ExpandTopicPartitionsInput>({
    mutationFn: expandTopicPartitions,
    onSuccess: async (result) => {
      const details = [
        `结果分区数：${result.resultingPartitionCount}`,
        result.auditRef ? `审计引用：${result.auditRef}` : '审计引用暂未写入，请查看后台日志。',
        result.warning ?? null,
      ].filter((item): item is string => Boolean(item));

      setFeedback({
        tone: result.warning ? 'warning' : 'success',
        message: `已提交 “${result.topicName}” 的分区扩容请求。`,
        details,
      });
      setLastPartitionExpansion({
        previousPartitionCount: result.previousPartitionCount,
        requestedPartitionCount: result.requestedPartitionCount,
        resultingPartitionCount: result.resultingPartitionCount,
        auditRef: result.auditRef,
        warning: result.warning,
      });
      setPartitionDraft(null);
      await queryClient.invalidateQueries({ queryKey: ['topic-detail', activeClusterProfileId, topicName] });
      await queryClient.invalidateQueries({ queryKey: ['topic-operations-overview', activeClusterProfileId, topicName] });
    },
    onError: (error) => {
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const activeDraftEntry = useMemo(() => {
    if (!draft) {
      return null;
    }

    return operationsOverviewQuery.data?.configEntries.find((entry) => entry.key === draft.configKey) ?? null;
  }, [draft, operationsOverviewQuery.data?.configEntries]);

  const activeDraftHasStaleConflict =
    Boolean(draft && activeDraftEntry && normalizeConfigEntryValue(activeDraftEntry.value) !== draft.expectedCurrentValue);

  const activeDraftEntryIsEditable = Boolean(activeDraftEntry && isEditableConfigEntry(activeDraftEntry));

  const activeDraftHasValueChange = Boolean(
    draft &&
      normalizeRequestedValueForSubmit(draft.configKey, draft.requestedValue) !==
        normalizeRequestedValueForSubmit(draft.configKey, draft.expectedCurrentValue),
  );

  const activeDraftValueInvalid = Boolean(
    draft &&
      ((draft.configKey === 'cleanup.policy' && cleanupPolicyState?.supported === false) ||
        (draft.configKey === 'cleanup.policy' && draft.requestedValue.trim().length === 0) ||
        (isNumericTopicConfigKey(draft.configKey) && !isDigitsOnly(draft.requestedValue.trim()))),
  );

  const activeDraftAdvisoryWarning =
    draft && isNumericTopicConfigKey(draft.configKey) && !activeDraftValueInvalid
      ? getNumericAdvisoryWarning(draft.configKey, draft.requestedValue.trim())
      : null;

  const activeDraftCanSubmit =
    Boolean(
      draft &&
        activeDraftEntry &&
        activeDraftEntryIsEditable &&
        !activeDraftHasStaleConflict &&
        activeDraftHasValueChange &&
        !activeDraftValueInvalid &&
        draft.riskAcknowledged &&
        draft.requestedValue.trim().length > 0 &&
        !updateMutation.isPending,
    );

  const currentPartitionCount = detailQuery.data?.topic.partitionCount ?? null;
  const partitionRequestedCount = partitionDraft ? Number(partitionDraft.requestedPartitionCount.trim()) : null;
  const partitionDraftValueInvalid = Boolean(
    partitionDraft &&
      (partitionDraft.requestedPartitionCount.trim().length === 0 ||
        !isDigitsOnly(partitionDraft.requestedPartitionCount.trim()) ||
        !Number.isSafeInteger(partitionRequestedCount) ||
        (partitionRequestedCount ?? 0) <= 0),
  );
  const partitionDraftHasStaleConflict = Boolean(
    partitionDraft &&
      currentPartitionCount !== null &&
      currentPartitionCount !== partitionDraft.expectedCurrentPartitionCount,
  );
  const partitionDraftHasIncrease = Boolean(
    partitionDraft &&
      !partitionDraftValueInvalid &&
      partitionRequestedCount !== null &&
      partitionRequestedCount > partitionDraft.expectedCurrentPartitionCount,
  );
  const partitionDraftCanSubmit = Boolean(
    partitionDraft &&
      !partitionDraftHasStaleConflict &&
      partitionDraftHasIncrease &&
      partitionDraft.riskAcknowledged &&
      !partitionExpansionMutation.isPending,
  );

  const handleOpenEditor = (entry: TopicOperationConfigEntry) => {
    if (updateMutation.isPending) {
      return;
    }

    if (draft?.configKey === entry.key) {
      return;
    }

    if (draft && (activeDraftHasValueChange || activeDraftHasStaleConflict)) {
      setFeedback({
        tone: 'warning',
        message: '当前有未保存或待处理的编辑内容，请先保存、取消，或关闭当前编辑器后再切换到其他配置项。',
      });
      return;
    }

    setDraft({
      configKey: entry.key,
      requestedValue: normalizeConfigEntryValue(entry.value),
      expectedCurrentValue: normalizeConfigEntryValue(entry.value),
      riskAcknowledged: false,
    });
    setFeedback(null);
  };

  const handleOpenPartitionExpansion = () => {
    if (partitionExpansionMutation.isPending || currentPartitionCount === null) {
      return;
    }

    setPartitionDraft({
      requestedPartitionCount: String(currentPartitionCount + 1),
      expectedCurrentPartitionCount: currentPartitionCount,
      riskAcknowledged: false,
    });
    setFeedback(null);
  };

  const handleSaveDraft = () => {
    if (!draft || !activeClusterProfileId || !decodedTopicName) {
      return;
    }

    if (updateMutation.isPending || activeDraftHasStaleConflict || !activeDraftEntryIsEditable) {
      return;
    }

    updateMutation.mutate({
      clusterProfileId: activeClusterProfileId,
      topicName: decodedTopicName,
      configKey: draft.configKey,
      requestedValue: normalizeRequestedValueForSubmit(draft.configKey, draft.requestedValue),
      expectedCurrentValue: draft.expectedCurrentValue,
      riskAcknowledged: draft.riskAcknowledged,
    });
  };

  const handleSavePartitionExpansion = () => {
    if (!partitionDraft || !activeClusterProfileId || !decodedTopicName || partitionRequestedCount === null) {
      return;
    }

    if (partitionExpansionMutation.isPending || !partitionDraftCanSubmit) {
      return;
    }

    partitionExpansionMutation.mutate({
      clusterProfileId: activeClusterProfileId,
      topicName: decodedTopicName,
      requestedPartitionCount: partitionRequestedCount,
      expectedCurrentPartitionCount: partitionDraft.expectedCurrentPartitionCount,
      riskAcknowledged: partitionDraft.riskAcknowledged,
    });
  };

  const handleToggleCleanupPolicy = (value: (typeof CLEANUP_POLICY_OPTIONS)[number], checked: boolean) => {
    setDraft((current) => {
      if (!current || updateMutation.isPending) {
        return current;
      }

      const parsed = parseCleanupPolicy(current.requestedValue);
      const nextValues = checked
        ? [...parsed.values, value]
        : parsed.values.filter((item) => item !== value);

      return {
        ...current,
        requestedValue: serializeCleanupPolicy(nextValues),
      };
    });
  };

  return (
    <PageFrame
      eyebrow="主题详情"
      title={decodedTopicName ?? '主题详情'}
      description="查看真实分区元数据，并继续进入消息排查。"
      contextualInfo={
        <div>
          <div className="workspace-note">当前集群与环境由全局 header 统一提供，详情页只保留返回与对象摘要。</div>
        </div>
      }
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

              <div className="workspace-block">
                <div className="workspace-section-label">运维能力概览</div>
                {operationsOverviewQuery.isLoading ? (
                  <div className="workspace-note">正在读取 Topic 运维能力…</div>
                ) : operationsOverviewQuery.isError ? (
                  <EmptyState
                    title="运维能力读取失败"
                    description={operationsOverviewQuery.error.message}
                    action={
                      <button
                        type="button"
                        className="button-shell"
                        data-variant="primary"
                        onClick={() => operationsOverviewQuery.refetch()}
                      >
                        重试
                      </button>
                    }
                  />
                ) : operationsOverviewQuery.data ? (
                  <div className="list-stack">
                    <div className="list-row">
                      <div>
                        <p className="list-row-title">整体状态</p>
                        <p className="list-row-meta">{operationsOverviewQuery.data.message}</p>
                      </div>
                      <Badge tone={toBadgeTone(operationsOverviewQuery.data.status)}>
                        {formatStageStatus(operationsOverviewQuery.data.status)}
                      </Badge>
                    </div>

                    <div className="workspace-block">
                      <div className="workspace-section-label">能力阶段</div>
                      <div className="list-stack">
                        {operationsOverviewQuery.data.stages.map((stage) => (
                          <div key={stage.key} className="list-row">
                            <div>
                              <p className="list-row-title">{stage.label}</p>
                              <p className="list-row-meta">{stage.message}</p>
                              {stage.detail ? <p className="list-row-meta mt-2">{stage.detail}</p> : null}
                            </div>
                            <Badge tone={toBadgeTone(stage.status)}>{formatStageStatus(stage.status)}</Badge>
                          </div>
                        ))}
                      </div>
                    </div>

                    <div className="workspace-block">
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <div className="workspace-section-label">分区扩容</div>
                          <div className="workspace-note">
                            只允许增加分区数；提交前会重新校验当前分区快照并要求风险确认。
                          </div>
                        </div>
                        {!partitionDraft ? (
                          <button
                            type="button"
                            className="button-shell"
                            data-variant="ghost"
                            disabled={partitionExpansionMutation.isPending || currentPartitionCount === null}
                            onClick={handleOpenPartitionExpansion}
                          >
                            扩容分区
                          </button>
                        ) : null}
                      </div>

                      {lastPartitionExpansion ? (
                        <div className="topic-config-applied-card mt-4">
                          <div className="topic-config-applied-head">
                            <div>
                              <p className="field-label">最近一次分区扩容</p>
                              <p className="topic-config-applied-title font-mono">{decodedTopicName}</p>
                            </div>
                            <Badge tone={lastPartitionExpansion.warning ? 'warning' : 'success'}>
                              {lastPartitionExpansion.warning ? '已提交但需关注' : '已提交'}
                            </Badge>
                          </div>
                          <div className="topic-config-compare-grid mt-3">
                            <div className="topic-config-compare-card" data-tone="current">
                              <p className="topic-config-compare-label">扩容前</p>
                              <p className="topic-config-compare-value font-mono">{lastPartitionExpansion.previousPartitionCount}</p>
                            </div>
                            <div className="topic-config-compare-divider">→</div>
                            <div className="topic-config-compare-card" data-tone="next">
                              <p className="topic-config-compare-label">扩容后</p>
                              <p className="topic-config-compare-value font-mono">{lastPartitionExpansion.resultingPartitionCount}</p>
                            </div>
                          </div>
                          <div className="topic-config-applied-meta mt-3">
                            <span>请求分区数：{lastPartitionExpansion.requestedPartitionCount}</span>
                            <span>审计引用：{lastPartitionExpansion.auditRef ?? '暂未写入'}</span>
                            {lastPartitionExpansion.warning ? <span>{lastPartitionExpansion.warning}</span> : null}
                          </div>
                        </div>
                      ) : null}

                      {partitionDraft ? (
                        <div className="mt-4 space-y-4">
                          <div className="topic-config-compare-grid">
                            <div className="topic-config-compare-card" data-tone="current">
                              <p className="topic-config-compare-label">当前分区数快照</p>
                              <p className="topic-config-compare-value font-mono">{partitionDraft.expectedCurrentPartitionCount}</p>
                              <p className="topic-config-compare-note">提交时会校验 Topic 仍然保持这个分区数。</p>
                            </div>
                            <div className="topic-config-compare-divider">→</div>
                            <div className="topic-config-compare-card" data-tone="next">
                              <p className="topic-config-compare-label">目标分区数</p>
                              <p className="topic-config-compare-value font-mono">{partitionDraft.requestedPartitionCount.trim() || '—'}</p>
                              <p className="topic-config-compare-note">Kafka 只支持增加分区数，不能降低分区数。</p>
                            </div>
                          </div>

                          {partitionDraftHasStaleConflict ? (
                            <div className="feedback-banner" data-tone="warning" role="status" aria-live="polite">
                              当前分区数已变化，请关闭扩容编辑器并重新打开后再提交。
                            </div>
                          ) : !partitionDraftHasIncrease ? (
                            <div className="feedback-banner" data-tone="muted" role="status" aria-live="polite">
                              目标分区数必须大于当前分区数快照。
                            </div>
                          ) : null}

                          <div className="form-grid">
                            <div>
                              <label className="field-label" htmlFor="topic-partition-requested-count">
                                目标分区数
                              </label>
                              <input
                                id="topic-partition-requested-count"
                                className="field-shell w-full font-mono"
                                type="text"
                                inputMode="numeric"
                                pattern="[0-9]*"
                                value={partitionDraft.requestedPartitionCount}
                                disabled={partitionExpansionMutation.isPending}
                                onChange={(event) =>
                                  setPartitionDraft((current) =>
                                    current && !partitionExpansionMutation.isPending
                                      ? { ...current, requestedPartitionCount: event.target.value }
                                      : current,
                                  )
                                }
                              />
                              {partitionDraftValueInvalid ? (
                                <div className="feedback-banner mt-3" data-tone="warning">
                                  这里只接受大于 0 的纯数字分区数。
                                </div>
                              ) : null}
                            </div>
                            <div>
                              <div className="field-label">提交时快照</div>
                              <div className="field-shell flex min-h-[2.75rem] w-full items-center font-mono text-sm text-ink-dim">
                                {partitionDraft.expectedCurrentPartitionCount}
                              </div>
                              <p className="workspace-note mt-2">该值来自打开扩容编辑器时读取到的当前分区数。</p>
                            </div>
                          </div>

                          <div className="list-row items-center">
                            <div>
                              <p className="list-row-title">风险确认</p>
                              <p className="list-row-meta">分区扩容不可回退，且会影响后续生产者 key 分布与消费者并行度。</p>
                            </div>
                            <button
                              type="button"
                              className="button-shell"
                              data-variant={partitionDraft.riskAcknowledged ? 'primary' : 'ghost'}
                              aria-pressed={partitionDraft.riskAcknowledged}
                              disabled={partitionExpansionMutation.isPending}
                              onClick={() =>
                                setPartitionDraft((current) =>
                                  current && !partitionExpansionMutation.isPending
                                    ? { ...current, riskAcknowledged: !current.riskAcknowledged }
                                    : current,
                                )
                              }
                            >
                              {partitionDraft.riskAcknowledged ? '已确认' : '未确认'}
                            </button>
                          </div>

                          <div className="workspace-actions justify-end">
                            <button
                              type="button"
                              className="button-shell"
                              data-variant="ghost"
                              disabled={partitionExpansionMutation.isPending}
                              onClick={() => {
                                setPartitionDraft(null);
                                setFeedback(null);
                              }}
                            >
                              取消
                            </button>
                            <button
                              type="button"
                              className="button-shell"
                              data-variant="primary"
                              disabled={!partitionDraftCanSubmit}
                              onClick={handleSavePartitionExpansion}
                            >
                              {partitionExpansionMutation.isPending ? '扩容中…' : '提交扩容'}
                            </button>
                          </div>
                        </div>
                      ) : null}
                    </div>

                    <div className="workspace-block">
                      <div className="workspace-section-label">配置探测</div>
                      <div className="list-stack">
                        {operationsOverviewQuery.data.configEntries.map((entry) => (
                          <div key={entry.key} className="list-row">
                            <div>
                              <p className="list-row-title font-mono">{entry.key}</p>
                              <p className="list-row-meta break-all">{formatConfigValue(entry)}</p>
                              <p className="list-row-meta mt-2">{describeConfigMetadata(entry)}</p>
                              {entry.note ? <p className="list-row-meta mt-2">{entry.note}</p> : null}
                            </div>
                            <div className="flex shrink-0 flex-col items-end gap-2">
                              <Badge tone={entry.isSupported ? 'signal' : 'muted'}>
                                {entry.isSupported ? '已探测' : '未返回'}
                              </Badge>
                              {isEditableConfigEntry(entry) ? (
                                <button
                                  type="button"
                                  className="button-shell"
                                  data-variant={draft?.configKey === entry.key ? 'primary' : 'ghost'}
                                  disabled={updateMutation.isPending}
                                  onClick={() => handleOpenEditor(entry)}
                                >
                                  {draft?.configKey === entry.key ? '正在编辑' : '编辑'}
                                </button>
                              ) : null}
                            </div>
                          </div>
                        ))}
                      </div>

                      {editableConfigEntries.length || draft || lastApplied ? (
                        <div className="workspace-block mt-4 rounded-2xl border border-line-subtle/60 bg-surface-2/80 p-4 shadow-sm">
                          <div className="flex items-start justify-between gap-3">
                            <div>
                              <div className="workspace-section-label">配置编辑器</div>
                              <div className="workspace-note">
                                只允许修改当前 broker 已确认可变更的配置项，并要求显式确认风险。
                              </div>
                            </div>
                            {draft ? (
                              <button
                                type="button"
                                className="button-shell"
                                data-variant="ghost"
                                disabled={updateMutation.isPending}
                                onClick={() => {
                                  setDraft(null);
                                  setFeedback(null);
                                }}
                              >
                                关闭编辑
                              </button>
                            ) : null}
                          </div>

                          {lastApplied ? (
                            <div className="topic-config-applied-card mt-4">
                              <div className="topic-config-applied-head">
                                <div>
                                  <p className="field-label">最近一次已应用修改</p>
                                  <p className="topic-config-applied-title font-mono">{lastApplied.configKey}</p>
                                </div>
                                <Badge tone={lastApplied.warning ? 'warning' : 'success'}>
                                  {lastApplied.warning ? '已应用但需关注' : '已应用'}
                                </Badge>
                              </div>
                              <div className="topic-config-compare-grid mt-3">
                                <div className="topic-config-compare-card" data-tone="current">
                                  <p className="topic-config-compare-label">应用前</p>
                                  <p className="topic-config-compare-value font-mono">{lastApplied.previousValue ?? '—'}</p>
                                </div>
                                <div className="topic-config-compare-divider">→</div>
                                <div className="topic-config-compare-card" data-tone="next">
                                  <p className="topic-config-compare-label">应用后</p>
                                  <p className="topic-config-compare-value font-mono">{lastApplied.resultingValue ?? '—'}</p>
                                </div>
                              </div>
                              <div className="topic-config-applied-meta mt-3">
                                <span>审计引用：{lastApplied.auditRef ?? '暂未写入'}</span>
                                {lastApplied.warning ? <span>{lastApplied.warning}</span> : null}
                              </div>
                            </div>
                          ) : null}

                          {draft && selectedConfigEntry && isEditableConfigEntry(selectedConfigEntry) ? (
                            <div className="mt-4 space-y-4">

                              <div className="list-row">
                                <div>
                                  <p className="list-row-title font-mono">{draft.configKey}</p>
                                  <p className="list-row-meta">当前值：{selectedConfigEntry.value ?? '—'}</p>
                                  <p className="list-row-meta mt-2">编辑时快照：{draft.expectedCurrentValue || '—'}</p>
                                </div>
                                <Badge tone="signal">{isNumericTopicConfigKey(draft.configKey) ? '数值型' : '文本型'}</Badge>
                              </div>

                              {activeDraftHasStaleConflict ? (
                                <div className="feedback-banner" data-tone="warning" role="status" aria-live="polite">
                                  当前值已变化，请先关闭编辑器并重新打开，以避免覆盖新的 broker 状态。
                                </div>
                              ) : !activeDraftHasValueChange ? (
                                <div className="feedback-banner" data-tone="muted" role="status" aria-live="polite">
                                  当前新值与编辑时快照一致，暂时没有需要提交的变更。
                                </div>
                              ) : null}

                              <div className="topic-config-compare-grid">
                                <div className="topic-config-compare-card" data-tone="current">
                                  <p className="topic-config-compare-label">当前 broker 值</p>
                                  <p className="topic-config-compare-value font-mono">{draft.expectedCurrentValue || '—'}</p>
                                  <p className="topic-config-compare-note">提交时会校验它仍然等于这个快照。</p>
                                </div>
                                <div className="topic-config-compare-divider">→</div>
                                <div className="topic-config-compare-card" data-tone="next">
                                  <p className="topic-config-compare-label">准备写入</p>
                                  <p className="topic-config-compare-value font-mono">{draft.requestedValue.trim() || '—'}</p>
                                  <p className="topic-config-compare-note">只会执行单个配置项的增量更新，不会替换整组配置。</p>
                                </div>
                              </div>

                              <div className="form-grid">
                                <div className="lg:col-span-8">
                                  {draft.configKey === 'cleanup.policy' ? (
                                    <fieldset className="topic-config-fieldset">
                                      <legend className="field-label">新值</legend>
                                      <div className="topic-config-chip-group">
                                        {CLEANUP_POLICY_OPTIONS.map((option) => {
                                          const checked = cleanupPolicyState?.values.includes(option) ?? false;
                                          return (
                                            <label
                                              key={option}
                                              className="topic-config-chip"
                                              data-selected={checked}
                                            >
                                              <input
                                                type="checkbox"
                                                value={option}
                                                checked={checked}
                                                disabled={updateMutation.isPending}
                                                onChange={(event) =>
                                                  handleToggleCleanupPolicy(option, event.target.checked)
                                                }
                                              />
                                              <span className="font-mono">{option}</span>
                                            </label>
                                          );
                                        })}
                                      </div>
                                      <p className="workspace-note mt-2">{describeEditableConfigHint(draft.configKey)}</p>
                                      {cleanupPolicyState?.supported === false ? (
                                        <div className="feedback-banner mt-3" data-tone="warning">
                                          当前 cleanup.policy 值包含未识别选项，请先回退为受支持组合后再提交。
                                        </div>
                                      ) : null}
                                    </fieldset>
                                  ) : (
                                    <>
                                      <label className="field-label" htmlFor="topic-config-requested-value">
                                        新值
                                      </label>
                                      <div className="topic-config-input-shell">
                                        <input
                                          id="topic-config-requested-value"
                                          className="field-shell w-full font-mono"
                                          type="text"
                                          inputMode={isNumericTopicConfigKey(draft.configKey) ? 'numeric' : undefined}
                                          pattern={isNumericTopicConfigKey(draft.configKey) ? '[0-9]*' : undefined}
                                          aria-invalid={isNumericTopicConfigKey(draft.configKey) && draft.requestedValue.trim().length > 0 && !isDigitsOnly(draft.requestedValue.trim())}
                                          value={draft.requestedValue}
                                          disabled={updateMutation.isPending}
                                          placeholder={
                                            draft.configKey === 'retention.ms'
                                              ? '例如 604800000'
                                              : draft.configKey === 'max.message.bytes'
                                                ? '例如 1048576'
                                                : '请输入新值'
                                          }
                                          onChange={(event) =>
                                            setDraft((current) =>
                                              current && !updateMutation.isPending
                                                ? {
                                                    ...current,
                                                    requestedValue: event.target.value,
                                                  }
                                                : current,
                                            )
                                          }
                                        />
                                        {draft.configKey === 'retention.ms' ? (
                                          <span className="topic-config-unit">ms</span>
                                        ) : draft.configKey === 'max.message.bytes' ? (
                                          <span className="topic-config-unit">bytes</span>
                                        ) : null}
                                      </div>
                                      <p className="workspace-note mt-2">{describeEditableConfigHint(draft.configKey)}</p>
                                      {isNumericTopicConfigKey(draft.configKey) ? (
                                        draft.requestedValue.trim().length > 0 && !isDigitsOnly(draft.requestedValue.trim()) ? (
                                          <div className="feedback-banner mt-3" data-tone="warning">
                                            这里只接受纯数字，请不要输入单位、空格或小数点。
                                          </div>
                                        ) : (
                                          <>
                                            <p className="workspace-note mt-2">
                                              {draft.configKey === 'retention.ms'
                                                ? formatDurationHint(draft.requestedValue.trim())
                                                : formatBytesHint(draft.requestedValue.trim())}
                                            </p>
                                            {activeDraftAdvisoryWarning ? (
                                              <div className="feedback-banner mt-3" data-tone="warning" role="status" aria-live="polite">
                                                {activeDraftAdvisoryWarning}
                                              </div>
                                            ) : null}
                                          </>
                                        )
                                      ) : null}
                                    </>
                                  )}
                                </div>

                                <div className="lg:col-span-4">
                                  <div className="field-label">编辑时快照</div>
                                  <div className="field-shell flex min-h-[2.75rem] w-full items-center font-mono text-sm text-ink-dim">
                                    {draft.expectedCurrentValue || '—'}
                                  </div>
                                  <p className="workspace-note mt-2">该快照由打开编辑器时的 broker 当前值生成，不允许手动修改。</p>
                                </div>
                              </div>

                              <div className="list-row items-center">
                                <div>
                                  <p className="list-row-title">风险确认</p>
                                  <p className="list-row-meta">提交前需要再次确认，这是一个真实的 broker 配置修改。</p>
                                </div>
                                <button
                                  type="button"
                                  className="button-shell"
                                  data-variant={draft.riskAcknowledged ? 'primary' : 'ghost'}
                                  aria-pressed={draft.riskAcknowledged}
                                  disabled={updateMutation.isPending}
                                  onClick={() =>
                                    setDraft((current) =>
                                      current && !updateMutation.isPending
                                        ? {
                                            ...current,
                                            riskAcknowledged: !current.riskAcknowledged,
                                          }
                                        : current,
                                    )
                                  }
                                >
                                  {draft.riskAcknowledged ? '已确认' : '未确认'}
                                </button>
                              </div>

                              <div className="workspace-actions justify-end">
                                <button
                                  type="button"
                                  className="button-shell"
                                  data-variant="ghost"
                                  disabled={updateMutation.isPending}
                                  onClick={() => {
                                    setDraft(null);
                                    setFeedback(null);
                                  }}
                                >
                                  取消
                                </button>
                                <button
                                  type="button"
                                  className="button-shell"
                                  data-variant="primary"
                                  disabled={!activeDraftCanSubmit || updateMutation.isPending}
                                  onClick={handleSaveDraft}
                                >
                                  {updateMutation.isPending ? '保存中…' : '保存修改'}
                                </button>
                              </div>
                            </div>
                          ) : draft && selectedConfigEntry ? (
                            <div className="workspace-note mt-3">
                              当前选中的配置项已不再处于可编辑状态，请关闭编辑器并重新确认 broker 返回的最新能力结果。
                            </div>
                          ) : draft ? (
                            <div className="workspace-note mt-3">
                              当前选中的配置项暂不可编辑，或其元数据尚未刷新完成。
                            </div>
                          ) : (
                            <div className="workspace-note mt-3">
                              选择一个可变更的配置项开始编辑。当前可编辑项：{editableConfigEntries.map((item) => item.key).join(' · ') || '无'}。
                            </div>
                          )}
                        </div>
                      ) : null}

                      {feedback ? (
                        <div
                          className="feedback-banner mt-3"
                          data-tone={feedback.tone}
                          role={feedback.tone === 'danger' ? 'alert' : 'status'}
                          aria-live="polite"
                        >
                          <div>{feedback.message}</div>
                          {feedback.details?.length ? (
                            <div className="mt-2 space-y-1 text-sm text-current/80">
                              {feedback.details.map((detail) => (
                                <div key={detail}>{detail}</div>
                              ))}
                            </div>
                          ) : null}
                        </div>
                      ) : null}
                    </div>
                  </div>
                ) : (
                  <EmptyState title="暂无运维能力概览" description="当前还没有可展示的 Topic 运维能力结果。" />
                )}
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
