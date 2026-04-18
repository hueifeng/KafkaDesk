import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { EmptyState } from '@/components/ui/empty-state';
import { getReplayPolicy, updateReplayPolicy } from '@/features/replay-policy/api';
import type { ReplayPolicy } from '@/features/replay-policy/types';
import type { AppError } from '@/lib/tauri';

const defaultReplayPolicy: ReplayPolicy = {
  allowLiveReplay: true,
  sandboxOnly: true,
  sandboxTopicPrefix: 'sandbox.',
  requireRiskAcknowledgement: true,
  deliveryTimeoutSeconds: 7,
  maxRetryAttempts: 1,
};

export function ReplayPolicyPage() {
  const queryClient = useQueryClient();
  const [formState, setFormState] = useState<ReplayPolicy>(defaultReplayPolicy);
  const [feedback, setFeedback] = useState<{ tone: 'success' | 'warning' | 'danger'; message: string } | null>(null);

  const replayPolicyQuery = useQuery<ReplayPolicy, AppError>({
    queryKey: ['replay-policy'],
    queryFn: getReplayPolicy,
  });

  useEffect(() => {
    if (replayPolicyQuery.data) {
      setFormState(replayPolicyQuery.data);
    }
  }, [replayPolicyQuery.data]);

  const saveMutation = useMutation({
    mutationFn: updateReplayPolicy,
    onSuccess: async (policy: ReplayPolicy) => {
      setFormState(policy);
      setFeedback({ tone: 'success', message: '回放策略已保存。' });
      queryClient.setQueryData(['replay-policy'], policy);
      await queryClient.invalidateQueries({ queryKey: ['replay-policy'] });
    },
    onError: (error: AppError) => {
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  return (
    <section className="workspace-surface">
      <div className="workspace-main">
        <div className="workspace-toolbar">
          <div>
            <div className="workspace-title">回放策略</div>
            <div className="workspace-note">把本地安全边界前置到设置里，由运行时统一执行，不让 UI 自己猜规则。</div>
          </div>
          <div className="workspace-actions">
            <button
              type="button"
              className="button-shell"
              data-variant="primary"
              disabled={replayPolicyQuery.isLoading || saveMutation.isPending}
              onClick={() => {
                setFeedback(null);
                saveMutation.mutate(formState);
              }}
            >
              {saveMutation.isPending ? '保存中…' : '保存策略'}
            </button>
          </div>
        </div>

        {replayPolicyQuery.isLoading ? (
          <div className="workspace-note py-6" role="status" aria-live="polite">正在加载回放策略…</div>
        ) : replayPolicyQuery.isError ? (
          <EmptyState title="回放策略加载失败" description={replayPolicyQuery.error.message} />
        ) : (
          <>
            <div className="list-stack">
              <div className="list-row">
                <div>
                  <p className="list-row-title">允许 Broker 投递回放</p>
                  <p className="list-row-meta">关闭后，运行时只允许 Dry Run，不接受 Broker 投递回放请求。</p>
                </div>
                <button type="button" className="button-shell" data-variant={formState.allowLiveReplay ? 'primary' : 'ghost'} aria-pressed={formState.allowLiveReplay} aria-label={formState.allowLiveReplay ? '关闭 Broker 投递回放' : '开启 Broker 投递回放'} onClick={() => setFormState((current) => ({ ...current, allowLiveReplay: !current.allowLiveReplay }))}>
                  {formState.allowLiveReplay ? '已开启' : '已关闭'}
                </button>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">只允许 Sandbox 目标</p>
                  <p className="list-row-meta">开启后，Broker 投递回放的目标主题必须匹配指定前缀。</p>
                </div>
                <button type="button" className="button-shell" data-variant={formState.sandboxOnly ? 'primary' : 'ghost'} aria-pressed={formState.sandboxOnly} aria-label={formState.sandboxOnly ? '关闭 Sandbox 主题限制' : '开启 Sandbox 主题限制'} onClick={() => setFormState((current) => ({ ...current, sandboxOnly: !current.sandboxOnly }))}>
                  {formState.sandboxOnly ? '已开启' : '已关闭'}
                </button>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">强制风险确认</p>
                  <p className="list-row-meta">保留显式确认步骤，避免把 Broker 投递回放做成单击即提交的危险动作。</p>
                </div>
                <button type="button" className="button-shell" data-variant={formState.requireRiskAcknowledgement ? 'primary' : 'ghost'} aria-pressed={formState.requireRiskAcknowledgement} aria-label={formState.requireRiskAcknowledgement ? '关闭风险确认要求' : '开启风险确认要求'} onClick={() => setFormState((current) => ({ ...current, requireRiskAcknowledgement: !current.requireRiskAcknowledgement }))}>
                  {formState.requireRiskAcknowledgement ? '已开启' : '已关闭'}
                </button>
              </div>
            </div>

            <div className="workspace-block mt-4">
              <label className="field-label" htmlFor="replay-policy-sandbox-prefix">Sandbox 主题前缀</label>
              <input id="replay-policy-sandbox-prefix" className="field-shell w-full font-mono" value={formState.sandboxTopicPrefix} onChange={(event) => setFormState((current) => ({ ...current, sandboxTopicPrefix: event.target.value }))} placeholder="例如 sandbox." />
            </div>

            <div className="workspace-block mt-4">
              <div className="form-grid">
                <div>
                  <label className="field-label" htmlFor="replay-policy-delivery-timeout">投递超时（秒）</label>
                  <input
                    id="replay-policy-delivery-timeout"
                    className="field-shell w-full"
                    type="number"
                    min={1}
                    max={60}
                    value={formState.deliveryTimeoutSeconds}
                    onChange={(event) => setFormState((current) => ({ ...current, deliveryTimeoutSeconds: Number(event.target.value || 0) }))}
                  />
                </div>
                <div>
                  <label className="field-label" htmlFor="replay-policy-max-retries">最大重试次数</label>
                  <input
                    id="replay-policy-max-retries"
                    className="field-shell w-full"
                    type="number"
                    min={1}
                    max={5}
                    value={formState.maxRetryAttempts}
                    onChange={(event) => setFormState((current) => ({ ...current, maxRetryAttempts: Number(event.target.value || 0) }))}
                  />
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
        <div className="workspace-section-label">当前策略解读</div>
        <div className="list-stack">
          <div className="list-row">
            <div>
              <p className="list-row-title">Broker 投递回放</p>
              <p className="list-row-meta">{formState.allowLiveReplay ? '允许，但仍受 Sandbox 前缀和风险确认限制。' : '已被策略禁止，仅允许 Dry Run。'}</p>
            </div>
          </div>
          <div className="list-row">
            <div>
              <p className="list-row-title">目标边界</p>
              <p className="list-row-meta">
                {formState.sandboxOnly ? `仅允许前缀为 ${formState.sandboxTopicPrefix || '（未设置）'} 的主题。` : '当前不限制主题前缀。'}
              </p>
            </div>
          </div>
          <div className="list-row">
            <div>
              <p className="list-row-title">执行参数</p>
              <p className="list-row-meta">单次投递超时 {formState.deliveryTimeoutSeconds}s，最多重试 {formState.maxRetryAttempts} 次。</p>
            </div>
          </div>
        </div>
      </aside>
    </section>
  );
}
