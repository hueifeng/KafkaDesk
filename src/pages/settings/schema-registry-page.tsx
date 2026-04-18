import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';
import {
  createSchemaRegistryProfile,
  listSchemaRegistryProfiles,
  testSchemaRegistryProfile,
  updateSchemaRegistryProfile,
} from '@/features/schema-registry/api';
import {
  emptySchemaRegistryProfileInput,
  type SchemaRegistryConnectionTestResponse,
  type SchemaRegistryProfile,
  type SchemaRegistryProfileInput,
  type SchemaRegistryProfileUpdateInput,
} from '@/features/schema-registry/types';
import type { AppError, ValidationStage } from '@/lib/tauri';

function toUpdateInput(id: string, current: SchemaRegistryProfileInput): SchemaRegistryProfileUpdateInput {
  return {
    id,
    ...current,
  };
}

function toFormState(profile: SchemaRegistryProfile): SchemaRegistryProfileInput {
  return {
    name: profile.name,
    baseUrl: profile.baseUrl,
    authMode: profile.authMode,
    credentialRef: profile.credentialRef ?? '',
    credentialSecret: '',
    notes: profile.notes ?? '',
  };
}

function toFeedbackTone(status: 'passed' | 'warning' | 'failed' | 'skipped'): 'success' | 'warning' | 'danger' {
  if (status === 'passed') {
    return 'success';
  }

  if (status === 'warning') {
    return 'warning';
  }

  return 'danger';
}

function toBadgeTone(status: ValidationStage['status']): 'success' | 'warning' | 'danger' | 'muted' {
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
      return 'muted';
  }
}

function toErrorTestResult(error: AppError): SchemaRegistryConnectionTestResponse {
  return {
    ok: false,
    status: 'failed',
    target: '不可用',
    message: error.message,
    stages: [
      {
        key: 'command-error',
        label: '运行时错误',
        status: 'failed',
        message: error.message,
        detail: `错误类别：${error.category} · 错误代码：${error.code}`,
        errorCategory: error.category,
        retriable: error.retriable,
      },
    ],
  };
}

function credentialSecretPlaceholder(authMode: SchemaRegistryProfileInput['authMode']) {
  if (authMode === 'basic') {
    return 'username:password（仅写入系统 keyring，不会落库）';
  }

  if (authMode === 'bearer') {
    return 'Bearer Token（仅写入系统 keyring，不会落库）';
  }

  return '当前认证方式无需 secret';
}

export function SchemaRegistryPage() {
  const queryClient = useQueryClient();
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const [formState, setFormState] = useState<SchemaRegistryProfileInput>(emptySchemaRegistryProfileInput);
  const [feedback, setFeedback] = useState<{ tone: 'success' | 'warning' | 'danger'; message: string } | null>(null);
  const [testResult, setTestResult] = useState<SchemaRegistryConnectionTestResponse | null>(null);

  const profilesQuery = useQuery<SchemaRegistryProfile[], AppError>({
    queryKey: ['schema-registry-profiles'],
    queryFn: listSchemaRegistryProfiles,
  });

  const sortedProfiles = useMemo(() => profilesQuery.data ?? [], [profilesQuery.data]);
  const selectedProfile = useMemo(
    () => sortedProfiles.find((profile) => profile.id === selectedProfileId) ?? null,
    [selectedProfileId, sortedProfiles],
  );

  useEffect(() => {
    if (!sortedProfiles.length) {
      setSelectedProfileId(null);
      return;
    }

    if (isCreatingNew) {
      return;
    }

    const activeProfile = selectedProfileId
      ? sortedProfiles.find((profile) => profile.id === selectedProfileId) ?? sortedProfiles[0]
      : sortedProfiles[0];

    if (selectedProfileId !== activeProfile.id) {
      setSelectedProfileId(activeProfile.id);
      return;
    }

    setFormState(toFormState(activeProfile));
  }, [isCreatingNew, selectedProfileId, sortedProfiles]);

  const createMutation = useMutation({
    mutationFn: createSchemaRegistryProfile,
    onSuccess: async (profile: SchemaRegistryProfile) => {
      setFeedback({ tone: 'success', message: `已创建模式注册表配置“${profile.name}”。` });
      setIsCreatingNew(false);
      setSelectedProfileId(profile.id);
      setFormState(toFormState(profile));
      await queryClient.invalidateQueries({ queryKey: ['schema-registry-profiles'] });
      await queryClient.invalidateQueries({ queryKey: ['clusters'] });
    },
    onError: (error: AppError) => {
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const updateMutation = useMutation({
    mutationFn: updateSchemaRegistryProfile,
    onSuccess: async (profile: SchemaRegistryProfile) => {
      setFeedback({ tone: 'success', message: `已保存模式注册表配置“${profile.name}”。` });
      setIsCreatingNew(false);
      setSelectedProfileId(profile.id);
      setFormState(toFormState(profile));
      await queryClient.invalidateQueries({ queryKey: ['schema-registry-profiles'] });
      await queryClient.invalidateQueries({ queryKey: ['clusters'] });
    },
    onError: (error: AppError) => {
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const testMutation = useMutation({
    mutationFn: testSchemaRegistryProfile,
    onSuccess: (result) => {
      setTestResult(result);
      setFeedback({ tone: toFeedbackTone(result.status), message: result.message });
    },
    onError: (error: AppError) => {
      setTestResult(toErrorTestResult(error));
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const isBusy = createMutation.isPending || updateMutation.isPending || testMutation.isPending;
  const formMode: 'create' | 'edit' = selectedProfileId && !isCreatingNew ? 'edit' : 'create';

  const handleSubmit = () => {
    setFeedback(null);

    if (formMode === 'edit' && selectedProfileId) {
      updateMutation.mutate(toUpdateInput(selectedProfileId, formState));
      return;
    }

    createMutation.mutate(formState);
  };

  const handleCreateNew = () => {
    setSelectedProfileId(null);
    setIsCreatingNew(true);
    setFormState(emptySchemaRegistryProfileInput);
    setFeedback(null);
    setTestResult(null);
  };

  return (
    <section className="workspace-surface">
      <div className="workspace-main">
        <div className="workspace-toolbar">
          <div>
            <div className="workspace-title">模式注册表</div>
            <div className="workspace-note">
              {selectedProfile ? `已选中 ${selectedProfile.name}` : formMode === 'create' ? '新建配置' : `共 ${sortedProfiles.length} 个配置`}
            </div>
          </div>
          <div className="workspace-actions">
            <button type="button" className="button-shell" data-variant="primary" onClick={handleCreateNew}>
              {formMode === 'create' ? '正在新建' : '新建配置'}
            </button>
            <button type="button" className="button-shell" data-variant="ghost" disabled={isBusy} onClick={() => testMutation.mutate({ ...formState, profileId: selectedProfileId })}>
              {testMutation.isPending ? '测试中…' : '测试连接'}
            </button>
            <button type="button" className="button-shell" data-variant="ghost" disabled={isBusy} onClick={handleSubmit}>
              {createMutation.isPending || updateMutation.isPending ? '保存中…' : formMode === 'edit' ? '保存修改' : '创建配置'}
            </button>
          </div>
        </div>

        {profilesQuery.isLoading ? (
          <div className="workspace-note py-6">正在加载模式注册表配置…</div>
        ) : profilesQuery.isError ? (
          <EmptyState title="模式注册表配置加载失败" description={profilesQuery.error.message} />
        ) : (
          <>
            <TableShell
              caption="模式注册表配置列表，包含名称、地址、认证方式、更新时间和选择操作。"
              columns={['名称', '地址', '认证', '最近更新', '操作']}
              emptyState={<EmptyState title="还没有模式注册表配置" description="先创建一个本地配置，用于后续消息解码。" />}
            >
              {sortedProfiles.map((profile) => (
                <tr key={profile.id}>
                  <td className="font-medium text-ink">{profile.name}</td>
                  <td className="font-mono text-xs text-ink-dim">{profile.baseUrl}</td>
                  <td>{profile.authMode}</td>
                  <td>{profile.updatedAt}</td>
                  <td>
                    <button
                      type="button"
                      className="button-shell"
                      data-variant={selectedProfileId === profile.id && !isCreatingNew ? 'primary' : 'ghost'}
                      aria-pressed={selectedProfileId === profile.id && !isCreatingNew}
                      aria-label={`选择模式注册表配置 ${profile.name}`}
                      onClick={() => {
                        setSelectedProfileId(profile.id);
                        setIsCreatingNew(false);
                        setFormState(toFormState(profile));
                        setFeedback(null);
                        setTestResult(null);
                      }}
                    >
                      选择
                    </button>
                  </td>
                </tr>
              ))}
            </TableShell>

            <div className="workspace-block mt-4">
              <div className="form-grid">
                <div>
                  <label className="field-label" htmlFor="schema-registry-name">名称</label>
                  <input id="schema-registry-name" className="field-shell w-full" value={formState.name} onChange={(event) => setFormState((current) => ({ ...current, name: event.target.value }))} placeholder="例如 Confluent Schema Registry" />
                </div>
                <div>
                  <label className="field-label" htmlFor="schema-registry-base-url">Base URL</label>
                  <input id="schema-registry-base-url" className="field-shell w-full font-mono" value={formState.baseUrl} onChange={(event) => setFormState((current) => ({ ...current, baseUrl: event.target.value }))} placeholder="https://schema-registry.internal:8081" />
                </div>
                <div>
                  <label className="field-label" htmlFor="schema-registry-auth-mode">认证方式</label>
                  <select id="schema-registry-auth-mode" className="field-shell w-full" value={formState.authMode} onChange={(event) => setFormState((current) => ({ ...current, authMode: event.target.value as SchemaRegistryProfileInput['authMode'] }))}>
                    <option value="none">无需认证</option>
                    <option value="basic">Basic Auth</option>
                    <option value="bearer">Bearer Token</option>
                  </select>
                </div>
                <div>
                  <label className="field-label" htmlFor="schema-registry-credential-ref">凭据引用</label>
                  <input id="schema-registry-credential-ref" className="field-shell w-full" value={formState.credentialRef ?? ''} onChange={(event) => setFormState((current) => ({ ...current, credentialRef: event.target.value }))} placeholder="可选，预留给安全存储引用" aria-describedby="schema-registry-credential-ref-hint" />
                  <p id="schema-registry-credential-ref-hint" className="mt-2 text-xs text-ink-muted">带认证模式下，KafkaDesk 会按这个 credentialRef 从系统 keyring 读取 secret。该引用会持久化保存，但 secret 本身不会写入数据库。</p>
                </div>
                <div>
                  <label className="field-label" htmlFor="schema-registry-credential-secret">凭据 Secret</label>
                  <input
                    id="schema-registry-credential-secret"
                    className="field-shell w-full"
                    type="password"
                    value={formState.credentialSecret ?? ''}
                    onChange={(event) => setFormState((current) => ({ ...current, credentialSecret: event.target.value }))}
                    placeholder={credentialSecretPlaceholder(formState.authMode)}
                    disabled={formState.authMode === 'none'}
                    aria-describedby="schema-registry-credential-secret-hint"
                  />
                  <p id="schema-registry-credential-secret-hint" className="mt-2 text-xs text-ink-muted">如果填写，这个值会写入系统 keyring，并在测试连接和运行时解码时优先使用；不填写则尝试读取既有 keyring 条目。</p>
                </div>
              </div>
              <div className="mt-3">
                <label className="field-label" htmlFor="schema-registry-notes">备注</label>
                <textarea id="schema-registry-notes" className="field-shell min-h-28 w-full" value={formState.notes ?? ''} onChange={(event) => setFormState((current) => ({ ...current, notes: event.target.value }))} placeholder="可选，记录用途、环境或认证说明" />
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
        <div className="workspace-section-label">连接状态</div>
        {testResult ? (
          <div className="list-stack">
            <div className="list-row">
              <div>
                <p className="list-row-title">结果</p>
                <p className="list-row-meta">{testResult.message}</p>
              </div>
              <Badge tone={toBadgeTone(testResult.status)}>{testResult.ok ? '已通过' : testResult.status === 'warning' ? '需处理' : '未通过'}</Badge>
            </div>
            <div className="list-row">
              <div>
                <p className="list-row-title">目标</p>
                <p className="list-row-meta font-mono">{testResult.target}</p>
              </div>
            </div>
            {testResult.stages.map((stage) => (
              <div key={stage.key} className="list-row">
                <div>
                  <p className="list-row-title">{stage.label}</p>
                  <p className="list-row-meta">{stage.message}</p>
                  {stage.detail ? <p className="mt-1 text-[0.72rem] text-ink-muted">{stage.detail}</p> : null}
                </div>
                <Badge tone={toBadgeTone(stage.status)}>
                  {stage.status === 'passed'
                    ? '通过'
                    : stage.status === 'warning'
                      ? '注意'
                      : stage.status === 'failed'
                        ? '失败'
                        : '跳过'}
                </Badge>
              </div>
            ))}
          </div>
        ) : (
          <EmptyState title="尚未测试连接" description="保存前先做一次连通性测试，确认主机和端口可达。" />
        )}
      </aside>
    </section>
  );
}
