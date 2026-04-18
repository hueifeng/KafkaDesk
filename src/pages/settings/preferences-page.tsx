import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { EmptyState } from '@/components/ui/empty-state';
import { listClusters } from '@/features/clusters/api';
import type { ClusterProfileSummary } from '@/features/clusters/types';
import { getAppPreferences, updateAppPreferences } from '@/features/preferences/api';
import type { AppPreferences } from '@/features/preferences/types';
import type { AppError } from '@/lib/tauri';

type FeedbackState = {
  tone: 'success' | 'warning' | 'danger';
  message: string;
};

const defaultFormState: AppPreferences = {
  preferredClusterId: null,
  tableDensity: 'compact',
  defaultMessageQueryWindowMinutes: 30,
  preferredTraceView: 'timeline',
};

export function PreferencesPage() {
  const queryClient = useQueryClient();
  const [formState, setFormState] = useState<AppPreferences>(defaultFormState);
  const [feedback, setFeedback] = useState<FeedbackState | null>(null);

  const preferencesQuery = useQuery<AppPreferences, AppError>({
    queryKey: ['app-preferences'],
    queryFn: getAppPreferences,
  });

  const clustersQuery = useQuery<ClusterProfileSummary[], AppError>({
    queryKey: ['clusters'],
    queryFn: listClusters,
  });

  useEffect(() => {
    if (preferencesQuery.data) {
      setFormState(preferencesQuery.data);
    }
  }, [preferencesQuery.data]);

  const saveMutation = useMutation({
    mutationFn: updateAppPreferences,
    onSuccess: async (preferences: AppPreferences) => {
      setFormState(preferences);
      setFeedback({ tone: 'success', message: '应用偏好已保存。' });
      queryClient.setQueryData(['app-preferences'], preferences);
      await queryClient.invalidateQueries({ queryKey: ['app-preferences'] });
    },
    onError: (error: AppError) => {
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const handleSave = () => {
    setFeedback(null);
    saveMutation.mutate({
      preferredClusterId: formState.preferredClusterId || null,
      tableDensity: formState.tableDensity,
      defaultMessageQueryWindowMinutes: Number(formState.defaultMessageQueryWindowMinutes),
      preferredTraceView: formState.preferredTraceView,
    });
  };

  return (
    <section className="workspace-surface" role="main">
      <div className="workspace-main">
        <div className="workspace-toolbar">
          <div>
            <div className="workspace-title">应用偏好</div>
            <div className="workspace-note">保存桌面工作台的默认视图、查询窗口与优先集群。</div>
          </div>
          <div className="workspace-actions">
            <button type="button" className="button-shell" data-variant="primary" onClick={handleSave} disabled={preferencesQuery.isLoading || saveMutation.isPending}>
              {saveMutation.isPending ? '正在保存…' : '保存偏好'}
            </button>
          </div>
        </div>

        {preferencesQuery.isLoading || clustersQuery.isLoading ? (
          <div className="workspace-note py-6">正在加载本地偏好…</div>
        ) : preferencesQuery.isError ? (
          <EmptyState title="偏好加载失败" description={preferencesQuery.error.message} />
        ) : clustersQuery.isError ? (
          <EmptyState title="集群列表加载失败" description={clustersQuery.error.message} />
        ) : (
          <>
            <div className="toolbar-shell mb-3">
              <div className="lg:col-span-6">
                <label className="field-label" htmlFor="preferences-message-window">默认消息查询窗口</label>
                <input
                  id="preferences-message-window"
                  className="field-shell w-full"
                  type="number"
                  min={5}
                  max={240}
                  step={5}
                  value={formState.defaultMessageQueryWindowMinutes}
                  onChange={(event) =>
                    setFormState((current) => ({
                      ...current,
                      defaultMessageQueryWindowMinutes: Number(event.target.value),
                    }))
                  }
                />
              </div>
              <div className="lg:col-span-6">
                <label className="field-label" htmlFor="preferences-table-density">表格密度</label>
                <select id="preferences-table-density" className="field-shell w-full" value={formState.tableDensity} onChange={(event) => setFormState((current) => ({ ...current, tableDensity: event.target.value as AppPreferences['tableDensity'] }))}>
                  <option value="compact">紧凑</option>
                  <option value="comfortable">舒适</option>
                </select>
              </div>
              <div className="lg:col-span-6">
                <label className="field-label" htmlFor="preferences-trace-view">默认追踪视图</label>
                <select id="preferences-trace-view" className="field-shell w-full" value={formState.preferredTraceView} onChange={(event) => setFormState((current) => ({ ...current, preferredTraceView: event.target.value as AppPreferences['preferredTraceView'] }))}>
                  <option value="timeline">时间线</option>
                  <option value="table">表格</option>
                </select>
              </div>
              <div className="lg:col-span-6">
                <label className="field-label" htmlFor="preferences-cluster">默认集群</label>
                <select id="preferences-cluster" className="field-shell w-full" value={formState.preferredClusterId ?? ''} onChange={(event) => setFormState((current) => ({ ...current, preferredClusterId: event.target.value || null }))}>
                  <option value="">不指定</option>
                  {(clustersQuery.data ?? []).map((cluster) => (
                    <option key={cluster.id} value={cluster.id}>
                      {cluster.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            {feedback ? (
              <div className="feedback-banner" data-tone={feedback.tone === 'success' ? 'success' : feedback.tone === 'warning' ? 'warning' : 'danger'} role={feedback.tone === 'danger' ? 'alert' : 'status'} aria-live="polite">
                {feedback.message}
              </div>
            ) : null}
          </>
        )}
      </div>

      <aside className="workspace-sidebar">
        <div className="workspace-section-label">当前说明</div>
        <div className="list-stack">
          <div className="list-row">
            <div>
              <p className="list-row-title">查询窗口</p>
              <p className="list-row-meta">控制消息查询页默认的时间窗口，避免每次手动重设。</p>
            </div>
          </div>
          <div className="list-row">
            <div>
              <p className="list-row-title">表格密度</p>
              <p className="list-row-meta">先做本地持久化，后续再逐步接入全局表格样式。</p>
            </div>
          </div>
          <div className="list-row">
            <div>
              <p className="list-row-title">默认集群</p>
              <p className="list-row-meta">用于后续启动时优先聚焦常用环境。</p>
            </div>
          </div>
        </div>
      </aside>
    </section>
  );
}
