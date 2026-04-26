import { act, type ReactElement } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createRoot, type Root } from 'react-dom/client';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_CLUSTER_NAME, useWorkbenchStore } from '@/app/workbench-store';
import { expandTopicPartitions, getTopicDetail, getTopicOperationsOverview, updateTopicConfig } from '@/features/topics/api';
import type {
  ExpandTopicPartitionsResponse,
  TopicDetailResponse,
  TopicOperationsOverviewResponse,
  UpdateTopicConfigResponse,
} from '@/features/topics/types';
import { TopicDetailPage } from '@/pages/topic-detail-page';

vi.mock('@/features/topics/api', () => ({
  getTopicDetail: vi.fn(),
  getTopicOperationsOverview: vi.fn(),
  updateTopicConfig: vi.fn(),
  expandTopicPartitions: vi.fn(),
  updateTopicTags: vi.fn(),
}));

const mockedGetTopicDetail = vi.mocked(getTopicDetail);
const mockedGetTopicOperationsOverview = vi.mocked(getTopicOperationsOverview);
const mockedUpdateTopicConfig = vi.mocked(updateTopicConfig);
const mockedExpandTopicPartitions = vi.mocked(expandTopicPartitions);

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Array<{ root: Root; container: HTMLDivElement }> = [];

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
}

async function renderWithProviders(ui: ReactElement, route = '/') {
  const container = document.createElement('div');
  document.body.appendChild(container);

  const root = createRoot(container);
  const queryClient = createTestQueryClient();
  mountedRoots.push({ root, container });

  await act(async () => {
    root.render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter
          future={{
            v7_startTransition: true,
            v7_relativeSplatPath: true,
          }}
          initialEntries={[route]}
        >
          {ui}
        </MemoryRouter>
      </QueryClientProvider>,
    );
  });

  return { container, queryClient };
}

async function waitFor(assertion: () => void, timeoutMs = 2_000) {
  const startedAt = Date.now();

  while (true) {
    try {
      assertion();
      return;
    } catch (error) {
      if (Date.now() - startedAt >= timeoutMs) {
        throw error;
      }

      await act(async () => {
        await new Promise((resolve) => window.setTimeout(resolve, 20));
      });
    }
  }
}

function requireElement<T extends Element>(selector: string) {
  const element = document.querySelector<T>(selector);
  expect(element, `Expected element matching selector: ${selector}`).not.toBeNull();
  return element as T;
}

function requireButtonByText(label: string) {
  const button = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).find((candidate) =>
    candidate.textContent?.trim().includes(label),
  );

  expect(button, `Expected button with label containing: ${label}`).not.toBeUndefined();
  return button as HTMLButtonElement;
}

async function click(element: HTMLElement) {
  await act(async () => {
    element.click();
  });
}

async function changeValue(element: HTMLInputElement, value: string) {
  await act(async () => {
    const prototype = Object.getPrototypeOf(element);
    const descriptor = Object.getOwnPropertyDescriptor(prototype, 'value');

    descriptor?.set?.call(element, value);
    element.dispatchEvent(new Event('input', { bubbles: true }));
    element.dispatchEvent(new Event('change', { bubbles: true }));
  });
}

function requireCheckboxByValue(value: string) {
  const checkbox = document.querySelector<HTMLInputElement>(`input[type="checkbox"][value="${value}"]`);
  expect(checkbox, `Expected checkbox for value: ${value}`).not.toBeNull();
  return checkbox as HTMLInputElement;
}

function createDeferredPromise<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;

  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}

function buildTopicDetailResponse(): TopicDetailResponse {
  return {
    topic: {
      name: 'orders',
      partitionCount: 6,
      replicationFactor: 3,
      schemaType: 'json',
      retentionSummary: '7d',
      activityHint: '活跃',
      isFavorite: false,
      tags: [],
    },
    partitions: [],
    relatedGroups: [],
    advancedConfig: [{ key: 'brokerBootstrap', value: 'localhost:9092' }],
  };
}

function buildConfigEntry(key: string, value: string) {
  return {
    key,
    value,
    isSupported: true,
    isReadOnly: false,
    isDefault: false,
    isSensitive: false,
    source: 'dynamic-topic',
    note: null,
  };
}

beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  useWorkbenchStore.setState({
    recentItems: [],
    activeClusterProfileId: 'cluster-1',
    activeClusterName: '开发集群',
    environment: 'dev',
    searchValue: '',
  });
});

afterEach(async () => {
  while (mountedRoots.length) {
    const mounted = mountedRoots.pop();
    if (!mounted) continue;

    await act(async () => {
      mounted.root.unmount();
    });
    mounted.container.remove();
  }

  document.body.innerHTML = '';
  useWorkbenchStore.setState({
    recentItems: [],
    activeClusterProfileId: null,
    activeClusterName: DEFAULT_CLUSTER_NAME,
    environment: 'local',
    searchValue: '',
  });
});

describe('TopicDetailPage', () => {
  it('renders advanced config summary and submits a fixed snapshot with write acknowledgement', async () => {
    const topicDetail = buildTopicDetailResponse();
    topicDetail.advancedConfig = [
      { key: 'brokerBootstrap', value: 'localhost:9092' },
      { key: 'retention.ms', value: '604800000' },
    ];

    const initialOverview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    const updatedOverview: TopicOperationsOverviewResponse = {
      ...initialOverview,
      configEntries: [buildConfigEntry('retention.ms', '86400000')],
    };

    const updateResponse: UpdateTopicConfigResponse = {
      topicName: 'orders',
      configKey: 'retention.ms',
      previousValue: '604800000',
      requestedValue: '86400000',
      resultingValue: '86400000',
      auditRef: 'audit-42',
      warning: null,
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValueOnce(initialOverview).mockResolvedValue(updatedOverview);
    mockedUpdateTopicConfig.mockResolvedValue(updateResponse);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('高级配置');
      expect(document.body.textContent).toContain('brokerBootstrap: localhost:9092');
      expect(document.body.textContent).toContain('retention.ms: 604800000');
    });

    await click(requireButtonByText('编辑'));

    await waitFor(() => {
      expect(requireElement<HTMLInputElement>('#topic-config-requested-value').value).toBe('604800000');
      expect(document.body.textContent).toContain('编辑时快照：604800000');
    });

    const saveButton = requireButtonByText('保存修改');
    expect(saveButton.disabled).toBe(true);

    await click(requireButtonByText('未确认'));
    expect(saveButton.disabled).toBe(true);

    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');

    await waitFor(() => {
      expect(saveButton.disabled).toBe(false);
    });

    await click(saveButton);

    await waitFor(() => {
      expect(mockedUpdateTopicConfig).toHaveBeenCalledTimes(1);
      expect(mockedUpdateTopicConfig.mock.calls[0]?.[0]).toEqual({
        clusterProfileId: 'cluster-1',
        topicName: 'orders',
        configKey: 'retention.ms',
        requestedValue: '86400000',
        expectedCurrentValue: '604800000',
        riskAcknowledged: true,
      });
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('已应用 “retention.ms” 的配置修改。');
      expect(document.body.textContent).toContain('结果值：86400000');
      expect(document.body.textContent).toContain('审计引用：audit-42');
      expect(document.body.textContent).toContain('最近一次已应用修改');
      expect(document.body.textContent).toContain('应用前');
      expect(document.body.textContent).toContain('应用后');
    });
  });

  it('submits partition expansion with a fixed partition-count snapshot and acknowledgement', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };
    const response: ExpandTopicPartitionsResponse = {
      topicName: 'orders',
      previousPartitionCount: 6,
      requestedPartitionCount: 8,
      resultingPartitionCount: 8,
      auditRef: 'audit-partitions',
      warning: null,
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);
    mockedExpandTopicPartitions.mockResolvedValue(response);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('分区扩容');
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('分区扩容');
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('分区扩容');
    });

    await click(requireButtonByText('扩容分区'));

    await waitFor(() => {
      expect(requireElement<HTMLInputElement>('#topic-partition-requested-count').value).toBe('7');
      expect(requireButtonByText('提交扩容').disabled).toBe(true);
    });

    await changeValue(requireElement<HTMLInputElement>('#topic-partition-requested-count'), '8');
    await click(requireButtonByText('未确认'));

    await waitFor(() => {
      expect(requireButtonByText('提交扩容').disabled).toBe(false);
    });

    await click(requireButtonByText('提交扩容'));

    await waitFor(() => {
      expect(mockedExpandTopicPartitions).toHaveBeenCalledTimes(1);
      expect(mockedExpandTopicPartitions.mock.calls[0]?.[0]).toEqual({
        clusterProfileId: 'cluster-1',
        topicName: 'orders',
        requestedPartitionCount: 8,
        expectedCurrentPartitionCount: 6,
        riskAcknowledged: true,
      });
      expect(document.body.textContent).toContain('已提交 “orders” 的分区扩容请求。');
      expect(document.body.textContent).toContain('最近一次分区扩容');
      expect(document.body.textContent).toContain('审计引用：audit-partitions');
    });
  });

  it('blocks partition expansion when the requested count is not greater than the snapshot', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('分区扩容');
    });

    await click(requireButtonByText('扩容分区'));
    await changeValue(requireElement<HTMLInputElement>('#topic-partition-requested-count'), '6');
    await click(requireButtonByText('未确认'));

    await waitFor(() => {
      expect(document.body.textContent).toContain('目标分区数必须大于当前分区数快照');
      expect(requireButtonByText('提交扩容').disabled).toBe(true);
    });

    expect(mockedExpandTopicPartitions).not.toHaveBeenCalled();
  });

  it('blocks partition expansion when the partition-count snapshot becomes stale', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    const { queryClient } = await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('分区扩容');
    });

    await click(requireButtonByText('扩容分区'));
    await changeValue(requireElement<HTMLInputElement>('#topic-partition-requested-count'), '8');
    await click(requireButtonByText('未确认'));

    await act(async () => {
      queryClient.setQueryData<TopicDetailResponse>(['topic-detail', 'cluster-1', 'orders'], {
        ...topicDetail,
        topic: { ...topicDetail.topic, partitionCount: 7 },
      });
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('当前分区数已变化');
      expect(requireButtonByText('提交扩容').disabled).toBe(true);
    });
  });

  it('keeps the recent-applied summary visible even when the refreshed editable set becomes empty', async () => {
    const topicDetail = buildTopicDetailResponse();
    const initialOverview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };
    const refreshedOverview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [
        {
          ...buildConfigEntry('retention.ms', '86400000'),
          isReadOnly: true,
        },
      ],
    };

    const updateResponse: UpdateTopicConfigResponse = {
      topicName: 'orders',
      configKey: 'retention.ms',
      previousValue: '604800000',
      requestedValue: '86400000',
      resultingValue: '86400000',
      auditRef: 'audit-empty-editable',
      warning: null,
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValueOnce(initialOverview).mockResolvedValue(refreshedOverview);
    mockedUpdateTopicConfig.mockResolvedValue(updateResponse);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');
    await click(requireButtonByText('保存修改'));

    await waitFor(() => {
      expect(document.body.textContent).toContain('最近一次已应用修改');
      expect(document.body.textContent).toContain('audit-empty-editable');
      expect(document.body.textContent).toContain('应用前');
      expect(document.body.textContent).toContain('应用后');
      expect(document.body.textContent).toContain('选择一个可变更的配置项开始编辑。当前可编辑项：无。');
    });
  });

  it('surfaces backend warnings on successful saves in feedback and applied summary', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('max.message.bytes', '1048576')],
    };
    const updateResponse: UpdateTopicConfigResponse = {
      topicName: 'orders',
      configKey: 'max.message.bytes',
      previousValue: '1048576',
      requestedValue: '999999999',
      resultingValue: '999999999',
      auditRef: 'audit-warning',
      warning: 'Kafka 已应用配置，但刷新后的 broker 元数据暂未确认最终值。',
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);
    mockedUpdateTopicConfig.mockResolvedValue(updateResponse);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('max.message.bytes');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '999999999');
    await click(requireButtonByText('保存修改'));

    await waitFor(() => {
      expect(document.body.textContent).toContain('已应用 “max.message.bytes” 的配置修改。');
      expect(document.body.textContent).toContain('已应用但需关注');
      expect(document.body.textContent).toContain('Kafka 已应用配置，但刷新后的 broker 元数据暂未确认最终值。');
      expect(document.body.textContent).toContain('审计引用：audit-warning');
      expect(document.body.textContent).toContain('最近一次已应用修改');
    });
  });

  it('does not silently replace an unsaved draft when another config edit is requested', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [
        buildConfigEntry('retention.ms', '604800000'),
        buildConfigEntry('max.message.bytes', '1048576'),
      ],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
      expect(document.body.textContent).toContain('max.message.bytes');
    });

    const editButtons = Array.from(document.querySelectorAll<HTMLButtonElement>('button')).filter((candidate) =>
      candidate.textContent?.trim().includes('编辑'),
    );
    expect(editButtons.length).toBeGreaterThanOrEqual(2);

    await click(editButtons[0]!);
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');

    await click(editButtons[1]!);

    await waitFor(() => {
      expect(document.body.textContent).toContain('当前有未保存或待处理的编辑内容');
      expect(requireElement<HTMLInputElement>('#topic-config-requested-value').value).toBe('86400000');
      expect(document.body.textContent).toContain('retention.ms');
    });

    expect(mockedUpdateTopicConfig).not.toHaveBeenCalled();
  });

  it('cancels a dirty draft without saving and reopens from the current broker value', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');
    await click(requireButtonByText('取消'));

    await waitFor(() => {
      expect(document.querySelector('#topic-config-requested-value')).toBeNull();
    });

    await click(requireButtonByText('编辑'));

    await waitFor(() => {
      expect(requireElement<HTMLInputElement>('#topic-config-requested-value').value).toBe('604800000');
      expect(requireButtonByText('未确认')).toBeTruthy();
      expect(requireButtonByText('保存修改').disabled).toBe(true);
    });

    expect(mockedUpdateTopicConfig).not.toHaveBeenCalled();
  });

  it('prevents duplicate submits and locks mutation-sensitive controls while save is pending', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };
    const pendingUpdate = createDeferredPromise<UpdateTopicConfigResponse>();

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);
    mockedUpdateTopicConfig.mockReturnValue(pendingUpdate.promise);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');

    const saveButton = requireButtonByText('保存修改');
    await waitFor(() => {
      expect(saveButton.disabled).toBe(false);
    });

    await click(saveButton);

    await waitFor(() => {
      expect(mockedUpdateTopicConfig).toHaveBeenCalledTimes(1);
      expect(requireButtonByText('保存中…').disabled).toBe(true);
      expect(requireButtonByText('关闭编辑').disabled).toBe(true);
      expect(requireButtonByText('已确认').disabled).toBe(true);
      expect(requireElement<HTMLInputElement>('#topic-config-requested-value').disabled).toBe(true);
    });

    await click(requireButtonByText('保存中…'));
    await click(requireButtonByText('已确认'));

    expect(mockedUpdateTopicConfig).toHaveBeenCalledTimes(1);
    expect(requireElement<HTMLInputElement>('#topic-config-requested-value').value).toBe('86400000');

    pendingUpdate.resolve({
      topicName: 'orders',
      configKey: 'retention.ms',
      previousValue: '604800000',
      requestedValue: '86400000',
      resultingValue: '86400000',
      auditRef: 'audit-pending',
      warning: null,
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('已应用 “retention.ms” 的配置修改。');
    });
  });

  it('blocks stale drafts after overview data changes until the editor is reopened', async () => {
    const topicDetail = buildTopicDetailResponse();
    const initialOverview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(initialOverview);

    const { queryClient } = await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');

    await act(async () => {
      queryClient.setQueryData<TopicOperationsOverviewResponse>(
        ['topic-operations-overview', 'cluster-1', 'orders'],
        {
          ...initialOverview,
          configEntries: [buildConfigEntry('retention.ms', '777777777')],
        },
      );
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('当前值已变化，请先关闭编辑器并重新打开');
      expect(requireButtonByText('保存修改').disabled).toBe(true);
    });

    expect(mockedUpdateTopicConfig).not.toHaveBeenCalled();

    await click(requireButtonByText('关闭编辑'));
    await click(requireButtonByText('编辑'));

    await waitFor(() => {
      expect(document.body.textContent).not.toContain('当前值已变化，请先关闭编辑器并重新打开');
      expect(requireElement<HTMLInputElement>('#topic-config-requested-value').value).toBe('777777777');
      expect(document.body.textContent).toContain('编辑时快照：777777777');
    });
  });

  it('keeps editable entries with empty current values editable instead of treating them as stale', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [
        {
          key: 'retention.ms',
          value: null,
          isSupported: true,
          isReadOnly: false,
          isDefault: false,
          isSensitive: false,
          source: 'dynamic-topic',
          note: null,
        },
      ],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));

    await waitFor(() => {
      expect(requireElement<HTMLInputElement>('#topic-config-requested-value').value).toBe('');
      expect(document.body.textContent).not.toContain('当前值已变化，请先关闭编辑器并重新打开');
    });

    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');

    await waitFor(() => {
      expect(requireButtonByText('保存修改').disabled).toBe(false);
    });
  });

  it('trims numeric values before submitting them to the backend', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    const updateResponse: UpdateTopicConfigResponse = {
      topicName: 'orders',
      configKey: 'retention.ms',
      previousValue: '604800000',
      requestedValue: '86400000',
      resultingValue: '86400000',
      auditRef: 'audit-trim',
      warning: null,
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);
    mockedUpdateTopicConfig.mockResolvedValue(updateResponse);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), ' 86400000 ');

    await waitFor(() => {
      expect(requireButtonByText('保存修改').disabled).toBe(false);
    });

    await click(requireButtonByText('保存修改'));

    await waitFor(() => {
      expect(mockedUpdateTopicConfig.mock.calls.at(-1)?.[0]).toEqual({
        clusterProfileId: 'cluster-1',
        topicName: 'orders',
        configKey: 'retention.ms',
        requestedValue: '86400000',
        expectedCurrentValue: '604800000',
        riskAcknowledged: true,
      });
    });
  });

  it('stops allowing submit when an open draft becomes read-only after overview refresh', async () => {
    const topicDetail = buildTopicDetailResponse();
    const initialOverview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(initialOverview);

    const { queryClient } = await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');

    await act(async () => {
      queryClient.setQueryData<TopicOperationsOverviewResponse>(
        ['topic-operations-overview', 'cluster-1', 'orders'],
        {
          ...initialOverview,
          configEntries: [
            {
              ...buildConfigEntry('retention.ms', '604800000'),
              isReadOnly: true,
            },
          ],
        },
      );
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('当前选中的配置项已不再处于可编辑状态');
    });

    expect(mockedUpdateTopicConfig).not.toHaveBeenCalled();
    expect(document.querySelector('#topic-config-requested-value')).toBeNull();
  });

  it('submits cleanup.policy through constrained checkbox controls', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('cleanup.policy', 'compact')],
    };

    const updateResponse: UpdateTopicConfigResponse = {
      topicName: 'orders',
      configKey: 'cleanup.policy',
      previousValue: 'compact',
      requestedValue: 'compact,delete',
      resultingValue: 'compact,delete',
      auditRef: 'audit-cleanup',
      warning: null,
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);
    mockedUpdateTopicConfig.mockResolvedValue(updateResponse);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('cleanup.policy');
    });

    await click(requireButtonByText('编辑'));

    await waitFor(() => {
      expect(requireCheckboxByValue('compact').checked).toBe(true);
      expect(requireCheckboxByValue('delete').checked).toBe(false);
    });

    await click(requireButtonByText('未确认'));
    await click(requireCheckboxByValue('delete'));

    await waitFor(() => {
      expect(document.body.textContent).toContain('准备写入compact,delete');
    });

    await click(requireButtonByText('保存修改'));

    await waitFor(() => {
      expect(mockedUpdateTopicConfig.mock.calls.at(-1)?.[0]).toEqual({
        clusterProfileId: 'cluster-1',
        topicName: 'orders',
        configKey: 'cleanup.policy',
        requestedValue: 'compact,delete',
        expectedCurrentValue: 'compact',
        riskAcknowledged: true,
      });
    });
  });

  it('keeps numeric config saves disabled for non-digit values', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('max.message.bytes', '1048576')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('max.message.bytes');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '10mb');

    await waitFor(() => {
      expect(document.body.textContent).toContain('这里只接受纯数字');
    });

    expect(requireButtonByText('保存修改').disabled).toBe(true);
    expect(mockedUpdateTopicConfig).not.toHaveBeenCalled();
  });

  it('shows advisory warning for suspicious but valid retention.ms values without blocking save', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '1000');

    await waitFor(() => {
      expect(document.body.textContent).toContain('这个 retention.ms 值明显偏短或偏长');
    });

    expect(requireButtonByText('保存修改').disabled).toBe(false);
  });

  it('clears numeric advisory warnings when the value returns to a normal valid range', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('retention.ms', '604800000')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '1000');

    await waitFor(() => {
      expect(document.body.textContent).toContain('这个 retention.ms 值明显偏短或偏长');
    });

    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');

    await waitFor(() => {
      expect(document.body.textContent).not.toContain('这个 retention.ms 值明显偏短或偏长');
      expect(requireButtonByText('保存修改').disabled).toBe(false);
    });
  });

  it('shows advisory warning for suspicious but valid max.message.bytes values without triggering invalid-number state', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [buildConfigEntry('max.message.bytes', '1048576')],
    };

    mockedGetTopicDetail.mockResolvedValue(topicDetail);
    mockedGetTopicOperationsOverview.mockResolvedValue(overview);

    await renderWithProviders(
      <Routes>
        <Route path="/topics/:topicName" element={<TopicDetailPage />} />
      </Routes>,
      '/topics/orders',
    );

    await waitFor(() => {
      expect(document.body.textContent).toContain('max.message.bytes');
    });

    await click(requireButtonByText('编辑'));
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '999999999');

    await waitFor(() => {
      expect(document.body.textContent).toContain('这个 max.message.bytes 值明显偏小或偏大');
    });

    expect(document.body.textContent).not.toContain('这里只接受纯数字');
    expect(requireButtonByText('保存修改').disabled).toBe(false);
  });
});
