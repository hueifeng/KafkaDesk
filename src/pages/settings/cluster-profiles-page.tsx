import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';
import { useWorkbenchStore } from '@/app/workbench-store';
import {
  createClusterProfile,
  getClusterProfile,
  listClusters,
  testClusterConnection,
  updateClusterProfile,
} from '@/features/clusters/api';
import { mapClusterToStorePayload } from '@/features/clusters/shared-helpers';
import { listSchemaRegistryProfiles } from '@/features/schema-registry/api';
import type { SchemaRegistryProfile } from '@/features/schema-registry/types';
import {
  emptyClusterProfileInput,
  type ClusterConnectionTestResponse,
  type ClusterProfileInput,
  type ClusterProfileSummary,
  type ClusterProfileUpdateInput,
} from '@/features/clusters/types';
import { getAppPreferences } from '@/features/preferences/api';
import type { AppPreferences } from '@/features/preferences/types';
import type { AppError, ValidationStage } from '@/lib/tauri';

const environmentOptions = ['dev', 'test', 'prod'] as const;
const authModeOptions = ['none', 'sasl-plain', 'sasl-scram', 'mtls'] as const;
const tlsModeOptions = ['system-default', 'tls-required', 'tls-insecure'] as const;

const environmentLabels: Record<(typeof environmentOptions)[number], string> = {
  dev: '开发',
  test: '测试',
  prod: '生产',
};

const authModeLabels: Record<(typeof authModeOptions)[number], string> = {
  none: '无需认证',
  'sasl-plain': 'SASL / PLAIN',
  'sasl-scram': 'SASL / SCRAM',
  mtls: '双向 TLS',
};

const tlsModeLabels: Record<(typeof tlsModeOptions)[number], string> = {
  'system-default': '系统默认',
  'tls-required': '强制 TLS',
  'tls-insecure': '允许不校验证书',
};

function toUpdateInput(selectedId: string, current: ClusterProfileInput): ClusterProfileUpdateInput {
  return {
    id: selectedId,
    ...current,
    isFavorite: false,
    isArchived: false,
  };
}

function toFormState(profile: {
  name: string;
  environment: string;
  bootstrapServers: string;
  authMode: string;
  authCredentialRef?: string | null;
  tlsMode: string;
  tlsCaCertPath?: string | null;
  tlsClientCertPath?: string | null;
  tlsClientKeyPath?: string | null;
  schemaRegistryProfileId?: string | null;
  notes?: string | null;
  tags?: string[];
}): ClusterProfileInput {
  return {
    name: profile.name,
    environment: normalizeEnvironment(profile.environment),
    bootstrapServers: profile.bootstrapServers,
    authMode: profile.authMode,
    authCredentialRef: profile.authCredentialRef ?? '',
    tlsMode: profile.tlsMode,
    tlsCaCertPath: profile.tlsCaCertPath ?? '',
    tlsClientCertPath: profile.tlsClientCertPath ?? '',
    tlsClientKeyPath: profile.tlsClientKeyPath ?? '',
    schemaRegistryProfileId: profile.schemaRegistryProfileId ?? null,
    notes: profile.notes ?? '',
    tags: profile.tags ?? [],
  };
}

function authSecretPlaceholder(authMode: string) {
  switch (authMode) {
    case 'sasl-plain':
    case 'sasl-scram':
      return 'username:password（仅写入系统 keyring，不会落库）';
    case 'mtls':
      return 'mTLS 不使用这里的 secret';
    default:
      return '当前认证方式无需 secret';
  }
}

function normalizeEnvironment(value: string): 'dev' | 'test' | 'prod' {
  if (value === 'test' || value === 'prod') {
    return value;
  }

  return 'dev';
}

function formatLastConnectedAt(value?: string | null) {
  if (!value) {
    return '未测试';
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(parsed);
}

function toFeedbackTone(status: 'passed' | 'warning' | 'failed' | 'skipped'): 'signal' | 'success' | 'warning' | 'danger' {
  if (status === 'passed') {
    return 'success';
  }

  if (status === 'warning') {
    return 'warning';
  }

  if (status === 'failed') {
    return 'danger';
  }

  return 'signal';
}

function toBadgeTone(status: ValidationStage['status']): 'success' | 'warning' | 'danger' | 'muted' | 'signal' {
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

function toErrorTestResult(error: AppError): ClusterConnectionTestResponse {
  return {
    ok: false,
    status: 'failed',
    attemptedBrokers: 0,
    reachableBrokers: 0,
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

export function ClusterProfilesPage() {
  const queryClient = useQueryClient();
  const setClusterContext = useWorkbenchStore((state) => state.setClusterContext);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [isCreatingNew, setIsCreatingNew] = useState(false);
  const [formState, setFormState] = useState<ClusterProfileInput>(emptyClusterProfileInput);
  const [feedback, setFeedback] = useState<{ tone: 'signal' | 'success' | 'warning' | 'danger'; message: string } | null>(null);
  const [testResult, setTestResult] = useState<ClusterConnectionTestResponse | null>(null);

  const formMode: 'create' | 'edit' = selectedProfileId && !isCreatingNew ? 'edit' : 'create';

  const clustersQuery = useQuery({
    queryKey: ['clusters'],
    queryFn: listClusters,
  });

  const preferencesQuery = useQuery<AppPreferences, AppError>({
    queryKey: ['app-preferences'],
    queryFn: getAppPreferences,
  });

  const schemaRegistryProfilesQuery = useQuery<SchemaRegistryProfile[], AppError>({
    queryKey: ['schema-registry-profiles'],
    queryFn: listSchemaRegistryProfiles,
  });

  const sortedProfiles = useMemo(() => clustersQuery.data ?? [], [clustersQuery.data]);
  const selectedProfile = useMemo(
    () => sortedProfiles.find((profile) => profile.id === selectedProfileId) ?? null,
    [selectedProfileId, sortedProfiles],
  );

  const selectedProfileDetailQuery = useQuery({
    queryKey: ['cluster-profile', selectedProfileId],
    queryFn: () => getClusterProfile(selectedProfileId!),
    enabled: Boolean(selectedProfileId) && !isCreatingNew,
  });

  useEffect(() => {
    if (!sortedProfiles.length) {
      setClusterContext({ activeClusterProfileId: null, activeClusterName: '新建集群配置', environment: 'local' });
      return;
    }

    if (isCreatingNew) {
      setClusterContext({ activeClusterProfileId: null, activeClusterName: '新建集群配置', environment: 'local' });
      return;
    }

    if (!selectedProfileId && preferencesQuery.data?.preferredClusterId) {
      const preferredProfile = sortedProfiles.find((profile) => profile.id === preferencesQuery.data?.preferredClusterId);
      if (preferredProfile) {
        setSelectedProfileId(preferredProfile.id);
        return;
      }
    }

    const activeProfile = selectedProfileId
      ? sortedProfiles.find((profile) => profile.id === selectedProfileId) ?? sortedProfiles[0]
      : sortedProfiles[0];

    if (selectedProfileId !== activeProfile.id) {
      setSelectedProfileId(activeProfile.id);
      return;
    }

    setClusterContext(mapClusterToStorePayload(activeProfile));
  }, [isCreatingNew, preferencesQuery.data?.preferredClusterId, selectedProfileId, setClusterContext, sortedProfiles]);

  useEffect(() => {
    if (!isCreatingNew && selectedProfileDetailQuery.data) {
      setFormState(toFormState(selectedProfileDetailQuery.data));
    }
  }, [isCreatingNew, selectedProfileDetailQuery.data]);

  const createMutation = useMutation({
    mutationFn: createClusterProfile,
    onSuccess: async (profile) => {
      setFeedback({ tone: 'success', message: `已创建集群配置“${profile.name}”。` });
      setIsCreatingNew(false);
      setSelectedProfileId(profile.id);
      setFormState(toFormState(profile));
      setClusterContext(mapClusterToStorePayload(profile));
      await queryClient.invalidateQueries({ queryKey: ['clusters'] });
      await queryClient.invalidateQueries({ queryKey: ['cluster-profile', profile.id] });
    },
    onError: (error: AppError) => {
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const updateMutation = useMutation({
    mutationFn: updateClusterProfile,
    onSuccess: async (profile) => {
      setFeedback({ tone: 'success', message: `已保存集群配置“${profile.name}”。` });
      setIsCreatingNew(false);
      setSelectedProfileId(profile.id);
      setFormState(toFormState(profile));
      setClusterContext(mapClusterToStorePayload(profile));
      await queryClient.invalidateQueries({ queryKey: ['clusters'] });
      await queryClient.invalidateQueries({ queryKey: ['cluster-profile', profile.id] });
    },
    onError: (error: AppError) => {
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const testMutation = useMutation({
    mutationFn: testClusterConnection,
    onSuccess: (result) => {
      setTestResult(result);
      setFeedback({ tone: toFeedbackTone(result.status), message: result.message });
      if (selectedProfileId) {
        void queryClient.invalidateQueries({ queryKey: ['clusters'] });
        void queryClient.invalidateQueries({ queryKey: ['cluster-profile', selectedProfileId] });
      }
    },
    onError: (error: AppError) => {
      setTestResult(toErrorTestResult(error));
      setFeedback({ tone: 'danger', message: error.message });
    },
  });

  const isBusy = createMutation.isPending || updateMutation.isPending || testMutation.isPending;

  const handleSubmit = () => {
    setFeedback(null);

    if (formMode === 'edit' && selectedProfileId) {
      updateMutation.mutate(toUpdateInput(selectedProfileId, formState));
      return;
    }

    createMutation.mutate(formState);
  };

  const handleProfileSelect = (profile: ClusterProfileSummary) => {
    setSelectedProfileId(profile.id);
    setIsCreatingNew(false);
    setFeedback(null);
    setTestResult(null);
    setClusterContext(mapClusterToStorePayload(profile));
  };

  const handleCreateNew = () => {
    setSelectedProfileId(null);
    setIsCreatingNew(true);
    setFormState(emptyClusterProfileInput);
    setFeedback(null);
    setTestResult(null);
    setClusterContext({ activeClusterProfileId: null, activeClusterName: '新建集群配置', environment: 'local' });
  };

  return (
    <section className="workspace-surface settings-cluster-workspace">
      <div className="workspace-main">
        <div className="workspace-toolbar">
          <div>
            <div className="workspace-title">集群配置</div>
            <div className="workspace-note">
              {selectedProfile ? `已选中 ${selectedProfile.name}` : formMode === 'create' ? '新建配置' : `共 ${sortedProfiles.length} 个配置`}
            </div>
          </div>
          <div className="workspace-actions">
            <button type="button" className="button-shell" data-variant="primary" onClick={handleCreateNew}>
              {formMode === 'create' ? '正在新建' : '新建配置'}
            </button>
            <button
              type="button"
              className="button-shell"
              data-variant="ghost"
              onClick={() => testMutation.mutate({ ...formState, profileId: formMode === 'edit' ? selectedProfileId : null })}
              disabled={isBusy}
            >
              {testMutation.isPending ? '测试中…' : '测试连接'}
            </button>
          </div>
        </div>

        {feedback ? (
          <div className="feedback-banner mb-3" data-tone={feedback.tone} role={feedback.tone === 'danger' ? 'alert' : 'status'} aria-live="polite">
            {feedback.message}
          </div>
        ) : null}

        <TableShell
          caption="集群配置列表，包含配置名称、环境、Bootstrap 地址、认证方式、TLS 模式和连接状态。"
          columns={['配置名称', '环境', 'Bootstrap 地址', '认证', 'TLS', '状态']}
          emptyState={
            <EmptyState
              title="还没有集群配置"
              description="先创建一个可用连接。"
              action={
                <button type="button" className="button-shell" data-variant="primary" onClick={handleCreateNew}>
                  创建首个配置
                </button>
              }
            />
          }
        >
          {sortedProfiles.map((profile) => {
            const active = selectedProfileId === profile.id && !isCreatingNew;
            const environment = normalizeEnvironment(profile.environment);

            return (
              <tr key={profile.id} className={active ? 'bg-elevated/70' : undefined}>
                <td>
                  <button
                    type="button"
                    className="w-full text-left text-ink transition hover:text-ink"
                    aria-pressed={active}
                    aria-label={`选择集群配置 ${profile.name}`}
                    onClick={() => handleProfileSelect(profile)}
                  >
                    <span className="font-medium">{profile.name}</span>
                    <span className="mt-1 block text-xs text-ink-muted">ID：{profile.id.slice(0, 8)}</span>
                  </button>
                </td>
                <td>
                  <Badge tone={environment === 'prod' ? 'warning' : 'signal'}>{environmentLabels[environment]}</Badge>
                </td>
                <td className="font-mono text-xs text-ink-dim">{profile.bootstrapServers}</td>
                <td>{authModeLabels[profile.authMode as keyof typeof authModeLabels] ?? profile.authMode}</td>
                <td>{tlsModeLabels[profile.tlsMode as keyof typeof tlsModeLabels] ?? profile.tlsMode}</td>
                <td>
                  <Badge tone={profile.lastConnectedAt ? 'success' : 'muted'}>
                    {profile.lastConnectedAt ? formatLastConnectedAt(profile.lastConnectedAt) : '未测试'}
                  </Badge>
                </td>
              </tr>
            );
          })}
        </TableShell>
      </div>

      <aside className="workspace-sidebar settings-cluster-sidebar">
        <div className="workspace-toolbar settings-cluster-editor-toolbar">
          <div>
            <div className="workspace-title">{formMode === 'edit' ? '编辑配置' : '新建配置'}</div>
            <div className="workspace-note">保存和测试都会沿用当前运行时命令。</div>
          </div>
          <div className="workspace-actions">
            <button type="button" className="button-shell" data-variant="primary" onClick={handleSubmit} disabled={isBusy}>
              {createMutation.isPending || updateMutation.isPending ? '保存中…' : formMode === 'edit' ? '保存' : '创建'}
            </button>
          </div>
        </div>

        <div className="workspace-block">
          <div className="form-grid">
            <div>
              <label className="field-label" htmlFor="cluster-profile-name">配置名称</label>
              <input
                id="cluster-profile-name"
                className="field-shell w-full"
                value={formState.name}
                onChange={(event) => setFormState((current) => ({ ...current, name: event.target.value }))}
              />
            </div>
            <div>
              <label className="field-label" htmlFor="cluster-profile-environment">环境</label>
              <select
                id="cluster-profile-environment"
                className="field-shell w-full"
                value={formState.environment}
                onChange={(event) => setFormState((current) => ({ ...current, environment: normalizeEnvironment(event.target.value) }))}
              >
                {environmentOptions.map((environment) => (
                  <option key={environment} value={environment}>
                    {environmentLabels[environment]}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>

        <div className="workspace-block">
          <div className="form-grid">
            <div>
              <label className="field-label" htmlFor="cluster-profile-auth-credential-ref">认证凭据引用</label>
              <input
                id="cluster-profile-auth-credential-ref"
                className="field-shell w-full"
                value={formState.authCredentialRef ?? ''}
                onChange={(event) => setFormState((current) => ({ ...current, authCredentialRef: event.target.value }))}
                placeholder="SASL 时必填，用于从系统 keyring 读取凭据"
                disabled={formState.authMode === 'none' || formState.authMode === 'mtls'}
              />
            </div>
            <div>
              <label className="field-label" htmlFor="cluster-profile-auth-secret">认证 Secret</label>
              <input
                id="cluster-profile-auth-secret"
                className="field-shell w-full"
                type="password"
                value={formState.authSecret ?? ''}
                onChange={(event) => setFormState((current) => ({ ...current, authSecret: event.target.value }))}
                placeholder={authSecretPlaceholder(formState.authMode)}
                disabled={formState.authMode === 'none' || formState.authMode === 'mtls'}
              />
            </div>
          </div>
          <p className="mt-3 text-xs text-ink-muted">SASL 模式下，填写的 Secret 会写入系统 keyring；数据库只保存凭据引用，不保存明文密码。</p>
        </div>

        <div className="workspace-block">
          <div className="form-grid">
            <div>
              <label className="field-label" htmlFor="cluster-profile-ca-cert-path">CA 证书路径</label>
              <input
                id="cluster-profile-ca-cert-path"
                className="field-shell w-full font-mono"
                value={formState.tlsCaCertPath ?? ''}
                onChange={(event) => setFormState((current) => ({ ...current, tlsCaCertPath: event.target.value }))}
                placeholder="可选：自定义 CA 证书路径"
              />
            </div>
            <div>
              <label className="field-label" htmlFor="cluster-profile-client-cert-path">客户端证书路径</label>
              <input
                id="cluster-profile-client-cert-path"
                className="field-shell w-full font-mono"
                value={formState.tlsClientCertPath ?? ''}
                onChange={(event) => setFormState((current) => ({ ...current, tlsClientCertPath: event.target.value }))}
                placeholder="mTLS 时必填：客户端证书文件路径"
                disabled={formState.authMode !== 'mtls'}
              />
            </div>
          </div>
          <div className="form-grid mt-3">
            <div>
              <label className="field-label" htmlFor="cluster-profile-client-key-path">客户端私钥路径</label>
              <input
                id="cluster-profile-client-key-path"
                className="field-shell w-full font-mono"
                value={formState.tlsClientKeyPath ?? ''}
                onChange={(event) => setFormState((current) => ({ ...current, tlsClientKeyPath: event.target.value }))}
                placeholder="mTLS 时必填：客户端私钥文件路径"
                disabled={formState.authMode !== 'mtls'}
              />
            </div>
          </div>
          <p className="mt-3 text-xs text-ink-muted">mTLS 模式下，当前版本使用本地文件路径装配证书和私钥；路径错误会在连接测试或回放执行时明确报错。</p>
        </div>

        <div className="workspace-block">
          <div className="form-grid">
            <div>
              <label className="field-label" htmlFor="cluster-profile-bootstrap-servers">Bootstrap 地址</label>
              <input
                id="cluster-profile-bootstrap-servers"
                className="field-shell w-full font-mono"
                value={formState.bootstrapServers}
                placeholder="broker-1:9092,broker-2:9092"
                onChange={(event) => setFormState((current) => ({ ...current, bootstrapServers: event.target.value }))}
              />
            </div>
            <div>
              <label className="field-label" htmlFor="cluster-profile-schema-registry">模式注册表</label>
              <select
                id="cluster-profile-schema-registry"
                className="field-shell w-full"
                value={formState.schemaRegistryProfileId ?? ''}
                onChange={(event) =>
                  setFormState((current) => ({
                    ...current,
                    schemaRegistryProfileId: event.target.value.trim() ? event.target.value : null,
                  }))
                }
              >
                <option value="">不关联</option>
                {(schemaRegistryProfilesQuery.data ?? []).map((profile) => (
                  <option key={profile.id} value={profile.id}>
                    {profile.name}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>

        <div className="workspace-block">
          <div className="form-grid">
            <div>
              <label className="field-label" htmlFor="cluster-profile-auth-mode">认证方式</label>
              <select
                id="cluster-profile-auth-mode"
                className="field-shell w-full"
                value={formState.authMode}
                onChange={(event) => setFormState((current) => ({ ...current, authMode: event.target.value }))}
              >
                {authModeOptions.map((authMode) => (
                  <option key={authMode} value={authMode}>
                    {authModeLabels[authMode]}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="field-label" htmlFor="cluster-profile-tls-mode">TLS 模式</label>
              <select
                id="cluster-profile-tls-mode"
                className="field-shell w-full"
                value={formState.tlsMode}
                onChange={(event) => setFormState((current) => ({ ...current, tlsMode: event.target.value }))}
              >
                {tlsModeOptions.map((tlsMode) => (
                  <option key={tlsMode} value={tlsMode}>
                    {tlsModeLabels[tlsMode]}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>

        <div className="workspace-block">
          <label className="field-label" htmlFor="cluster-profile-notes">备注</label>
          <textarea
            id="cluster-profile-notes"
            className="field-shell min-h-24 w-full resize-y"
            value={formState.notes ?? ''}
            placeholder="可选：记录 VPN、白名单或负责人"
            onChange={(event) => setFormState((current) => ({ ...current, notes: event.target.value }))}
          />
        </div>

        <div className="workspace-block">
          <div className="workspace-section-label">当前状态</div>
          <div className="list-stack">
            <div className="list-row">
              <div>
                <p className="list-row-title">当前对象</p>
                <p className="list-row-meta">{selectedProfile ? selectedProfile.name : '新配置草稿'}</p>
              </div>
              <Badge tone={formMode === 'edit' ? 'signal' : 'muted'}>{formMode === 'edit' ? '编辑中' : '未保存'}</Badge>
            </div>
            <div className="list-row">
              <div>
                <p className="list-row-title">环境 / 安全</p>
                <p className="list-row-meta">
                  {environmentLabels[normalizeEnvironment(formState.environment)]} / {authModeLabels[formState.authMode as keyof typeof authModeLabels] ?? formState.authMode} / {tlsModeLabels[formState.tlsMode as keyof typeof tlsModeLabels] ?? formState.tlsMode}
                </p>
              </div>
            </div>
            <div className="list-row">
              <div>
                <p className="list-row-title">连接测试</p>
                <p className="list-row-meta">{testResult ? testResult.message : '尚未执行测试。'}</p>
                {testResult ? (
                  <p className="mt-1 text-[0.72rem] text-ink-muted">
                    已响应 {testResult.reachableBrokers} / {testResult.attemptedBrokers} 个 Bootstrap 地址。
                  </p>
                ) : null}
              </div>
              <Badge tone={testResult ? toBadgeTone(testResult.status) : 'muted'}>
                {testResult ? (testResult.ok ? '已通过' : testResult.status === 'warning' ? '需处理' : '未通过') : '未测试'}
              </Badge>
            </div>
            {testResult?.stages.map((stage) => (
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
        </div>
      </aside>
    </section>
  );
}
