import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';
import { listClusters } from '@/features/clusters/api';
import type { ClusterProfileSummary } from '@/features/clusters/types';
import { createCorrelationRule, listCorrelationRules, updateCorrelationRule } from '@/features/correlation/api';
import {
  emptyCorrelationRuleInput,
  type CorrelationRule,
  type CorrelationRuleInput,
  type CorrelationStrategy,
} from '@/features/correlation/types';
import type { AppError } from '@/lib/tauri';

const strategyOptions: Array<{ value: CorrelationStrategy; label: string }> = [
  { value: 'header-match', label: 'Header 匹配' },
  { value: 'key-match', label: 'Key 匹配' },
  { value: 'decoded-field-match', label: '解码字段匹配' },
  { value: 'ordered-multi-topic', label: '多主题顺序关联' },
];

function toFormState(rule: CorrelationRule): CorrelationRuleInput {
  return {
    name: rule.name,
    clusterProfileId: rule.clusterProfileId,
    isEnabled: rule.isEnabled,
    matchStrategy: rule.matchStrategy,
    scopeJson: rule.scopeJson,
    ruleJson: rule.ruleJson,
  };
}

export function CorrelationRulesPage() {
  const queryClient = useQueryClient();
  const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const [formState, setFormState] = useState<CorrelationRuleInput>(emptyCorrelationRuleInput);
  const [feedback, setFeedback] = useState<{ tone: 'success' | 'warning' | 'danger'; message: string } | null>(null);

  const rulesQuery = useQuery<CorrelationRule[], AppError>({
    queryKey: ['correlation-rules'],
    queryFn: listCorrelationRules,
  });

  const clustersQuery = useQuery<ClusterProfileSummary[], AppError>({
    queryKey: ['clusters'],
    queryFn: listClusters,
  });

  const sortedRules = useMemo(() => rulesQuery.data ?? [], [rulesQuery.data]);
  const selectedRule = useMemo(
    () => sortedRules.find((rule) => rule.id === selectedRuleId) ?? null,
    [selectedRuleId, sortedRules],
  );

  useEffect(() => {
    if (!sortedRules.length) {
      setSelectedRuleId(null);
      return;
    }

    if (isCreatingNew) {
      return;
    }

    const activeRule = selectedRuleId ? sortedRules.find((rule) => rule.id === selectedRuleId) ?? sortedRules[0] : sortedRules[0];

    if (selectedRuleId !== activeRule.id) {
      setSelectedRuleId(activeRule.id);
      return;
    }

    setFormState(toFormState(activeRule));
  }, [isCreatingNew, selectedRuleId, sortedRules]);

  useEffect(() => {
    if (!formState.clusterProfileId && clustersQuery.data?.length && isCreatingNew) {
      setFormState((current) => ({ ...current, clusterProfileId: clustersQuery.data?.[0]?.id ?? '' }));
    }
  }, [clustersQuery.data, formState.clusterProfileId, isCreatingNew]);

  const createMutation = useMutation({
    mutationFn: createCorrelationRule,
    onSuccess: async (rule: CorrelationRule) => {
      setFeedback({ tone: 'success', message: `已创建关联规则“${rule.name}”。` });
      setIsCreatingNew(false);
      setSelectedRuleId(rule.id);
      setFormState(toFormState(rule));
      await queryClient.invalidateQueries({ queryKey: ['correlation-rules'] });
    },
    onError: (error: AppError) => setFeedback({ tone: 'danger', message: error.message }),
  });

  const updateMutation = useMutation({
    mutationFn: updateCorrelationRule,
    onSuccess: async (rule: CorrelationRule) => {
      setFeedback({ tone: 'success', message: `已保存关联规则“${rule.name}”。` });
      setIsCreatingNew(false);
      setSelectedRuleId(rule.id);
      setFormState(toFormState(rule));
      await queryClient.invalidateQueries({ queryKey: ['correlation-rules'] });
    },
    onError: (error: AppError) => setFeedback({ tone: 'danger', message: error.message }),
  });

  const isBusy = createMutation.isPending || updateMutation.isPending;
  const formMode: 'create' | 'edit' = selectedRuleId && !isCreatingNew ? 'edit' : 'create';

  return (
    <section className="workspace-surface">
      <div className="workspace-main">
        <div className="workspace-toolbar">
          <div>
            <div className="workspace-title">关联规则</div>
            <div className="workspace-note">
              {selectedRule ? `已选中 ${selectedRule.name}` : formMode === 'create' ? '新建规则' : `共 ${sortedRules.length} 条规则`}
            </div>
          </div>
          <div className="workspace-actions">
            <button
              type="button"
              className="button-shell"
              data-variant="primary"
              onClick={() => {
                setSelectedRuleId(null);
                setIsCreatingNew(true);
                setFormState({
                  ...emptyCorrelationRuleInput,
                  clusterProfileId: clustersQuery.data?.[0]?.id ?? '',
                });
                setFeedback(null);
              }}
            >
              {formMode === 'create' ? '正在新建' : '新建规则'}
            </button>
            <button
              type="button"
              className="button-shell"
              data-variant="ghost"
              disabled={isBusy}
              onClick={() => {
                setFeedback(null);
                if (formMode === 'edit' && selectedRuleId) {
                  updateMutation.mutate({ id: selectedRuleId, ...formState });
                  return;
                }

                createMutation.mutate(formState);
              }}
            >
              {isBusy ? '保存中…' : formMode === 'edit' ? '保存修改' : '创建规则'}
            </button>
          </div>
        </div>

        {rulesQuery.isLoading || clustersQuery.isLoading ? (
          <div className="workspace-note py-6">正在加载关联规则…</div>
        ) : rulesQuery.isError ? (
          <EmptyState title="关联规则加载失败" description={rulesQuery.error.message} />
        ) : clustersQuery.isError ? (
          <EmptyState title="集群列表加载失败" description={clustersQuery.error.message} />
        ) : (
          <>
            <TableShell
              caption="关联规则列表，包含名称、集群范围、策略、启用状态、更新时间和选择操作。"
              columns={['名称', '集群范围', '策略', '状态', '最近更新', '操作']}
              emptyState={<EmptyState title="还没有关联规则" description="先建立一条本地规则，为后续 trace 工作流做准备。" />}
            >
              {sortedRules.map((rule) => {
                const clusterName = clustersQuery.data?.find((cluster) => cluster.id === rule.clusterProfileId)?.name ?? rule.clusterProfileId;

                return (
                  <tr key={rule.id}>
                    <td className="font-medium text-ink">{rule.name}</td>
                    <td>{clusterName}</td>
                    <td>{strategyOptions.find((option) => option.value === rule.matchStrategy)?.label ?? rule.matchStrategy}</td>
                    <td>{rule.isEnabled ? '已启用' : '已停用'}</td>
                    <td>{rule.updatedAt}</td>
                    <td>
                      <button
                        type="button"
                        className="button-shell"
                        data-variant={selectedRuleId === rule.id && !isCreatingNew ? 'primary' : 'ghost'}
                        aria-pressed={selectedRuleId === rule.id && !isCreatingNew}
                        aria-label={`选择关联规则 ${rule.name}`}
                        onClick={() => {
                          setSelectedRuleId(rule.id);
                          setIsCreatingNew(false);
                          setFormState(toFormState(rule));
                          setFeedback(null);
                        }}
                      >
                        选择
                      </button>
                    </td>
                  </tr>
                );
              })}
            </TableShell>

            <div className="workspace-block mt-4">
              <div className="form-grid">
                <div>
                  <label className="field-label" htmlFor="correlation-rule-name">规则名称</label>
                  <input id="correlation-rule-name" className="field-shell w-full" value={formState.name} onChange={(event) => setFormState((current) => ({ ...current, name: event.target.value }))} placeholder="例如 TraceId Header 关联" />
                </div>
                <div>
                  <label className="field-label" htmlFor="correlation-rule-cluster">集群范围</label>
                  <select id="correlation-rule-cluster" className="field-shell w-full" value={formState.clusterProfileId} onChange={(event) => setFormState((current) => ({ ...current, clusterProfileId: event.target.value }))}>
                    <option value="">请选择集群</option>
                    {(clustersQuery.data ?? []).map((cluster) => (
                      <option key={cluster.id} value={cluster.id}>
                        {cluster.name}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="field-label" htmlFor="correlation-rule-strategy">策略类型</label>
                  <select id="correlation-rule-strategy" className="field-shell w-full" value={formState.matchStrategy} onChange={(event) => setFormState((current) => ({ ...current, matchStrategy: event.target.value as CorrelationStrategy }))}>
                    {strategyOptions.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="field-label" htmlFor="correlation-rule-enabled">启用状态</label>
                  <button id="correlation-rule-enabled" type="button" className="button-shell w-full justify-center" data-variant={formState.isEnabled ? 'primary' : 'ghost'} aria-pressed={formState.isEnabled} onClick={() => setFormState((current) => ({ ...current, isEnabled: !current.isEnabled }))}>
                    {formState.isEnabled ? '已启用' : '已停用'}
                  </button>
                </div>
              </div>

              <div className="form-grid mt-4">
                <div>
                  <label className="field-label" htmlFor="correlation-rule-scope-json">Scope JSON</label>
                  <textarea id="correlation-rule-scope-json" className="field-shell min-h-40 w-full font-mono text-xs leading-6" value={formState.scopeJson} onChange={(event) => setFormState((current) => ({ ...current, scopeJson: event.target.value }))} />
                </div>
                <div>
                  <label className="field-label" htmlFor="correlation-rule-json">Rule JSON</label>
                  <textarea id="correlation-rule-json" className="field-shell min-h-40 w-full font-mono text-xs leading-6" value={formState.ruleJson} onChange={(event) => setFormState((current) => ({ ...current, ruleJson: event.target.value }))} />
                </div>
              </div>
            </div>

            {feedback ? (
              <div className="feedback-banner mt-3" data-tone={feedback.tone === 'success' ? 'success' : feedback.tone === 'warning' ? 'warning' : 'danger'} role={feedback.tone === 'danger' ? 'alert' : 'status'} aria-live="polite">
                {feedback.message}
              </div>
            ) : null}
          </>
        )}
      </div>

      <aside className="workspace-sidebar">
        <div className="workspace-section-label">当前范围</div>
        <div className="list-stack">
          <div className="list-row">
            <div>
              <p className="list-row-title">这是规则底座</p>
              <p className="list-row-meta">当前只做本地规则配置与持久化，不假装已经完成真实 Trace 执行引擎。</p>
            </div>
          </div>
          <div className="list-row">
            <div>
              <p className="list-row-title">建议先配 Header / Key</p>
              <p className="list-row-meta">优先配置 traceId、orderId 等高信号字段，为后续 trace-by-key 做准备。</p>
            </div>
          </div>
        </div>
      </aside>
    </section>
  );
}
