import { act, type ReactElement } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createRoot, type Root } from 'react-dom/client';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_CLUSTER_NAME, useWorkbenchStore } from '@/app/workbench-store';
import { getTopicDetail, getTopicOperationsOverview, updateTopicConfig } from '@/features/topics/api';
import { TopicDetailPage } from '@/pages/topic-detail-page';
import type { TopicDetailResponse, TopicOperationsOverviewResponse, UpdateTopicConfigResponse } from '@/features/topics/types';

vi.mock('@/features/topics/api', () => ({
  getTopicDetail: vi.fn(),
  getTopicOperationsOverview: vi.fn(),
  updateTopicConfig: vi.fn(),
}));

const mockedGetTopicDetail = vi.mocked(getTopicDetail);
const mockedGetTopicOperationsOverview = vi.mocked(getTopicOperationsOverview);
const mockedUpdateTopicConfig = vi.mocked(updateTopicConfig);

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

beforeEach(() => {
  vi.clearAllMocks();
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
    },
    partitions: [],
    relatedGroups: [],
    config: [],
  };
}

describe('TopicDetailPage', () => {
  it('submits a fixed snapshot and surfaces the write acknowledgement', async () => {
    const topicDetail = buildTopicDetailResponse();
    topicDetail.config = [{ key: 'brokerBootstrap', value: 'localhost:9092' }];

    const initialOverview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [
        {
          key: 'retention.ms',
          value: '604800000',
          isSupported: true,
          isReadOnly: false,
          isDefault: false,
          isSensitive: false,
          source: 'dynamic-topic',
          note: null,
        },
      ],
    };

    const updatedOverview: TopicOperationsOverviewResponse = {
      ...initialOverview,
      configEntries: [{ ...initialOverview.configEntries[0], value: '86400000' }],
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
      expect(document.body.textContent).toContain('运维能力概览');
      expect(document.body.textContent).toContain('retention.ms');
    });

    await click(requireButtonByText('编辑'));

    await waitFor(() => {
      expect(requireElement<HTMLInputElement>('#topic-config-requested-value').value).toBe('604800000');
      expect(document.querySelector('#topic-config-expected-value')).toBeNull();
      expect(document.body.textContent).toContain('编辑时快照：604800000');
    });

    const saveButton = requireButtonByText('保存修改');
    expect(saveButton.disabled).toBe(true);

    await click(requireButtonByText('未确认'));
    expect(saveButton.disabled).toBe(true);
    expect(document.body.textContent).toContain('当前新值与编辑时快照一致，暂时没有需要提交的变更。');

    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '86400000');
    expect(saveButton.disabled).toBe(false);

    await click(saveButton);

    await waitFor(() => {
      expect(mockedUpdateTopicConfig).toHaveBeenCalled();
      expect(mockedUpdateTopicConfig.mock.calls.at(-1)?.[0]).toEqual({
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
    });
  });

  it('submits cleanup.policy through constrained checkbox controls', async () => {
    const topicDetail = buildTopicDetailResponse();
    const initialOverview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [
        {
          key: 'cleanup.policy',
          value: 'compact',
          isSupported: true,
          isReadOnly: false,
          isDefault: false,
          isSensitive: false,
          source: 'dynamic-topic',
          note: null,
        },
      ],
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
    mockedGetTopicOperationsOverview.mockResolvedValue(initialOverview);
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
      expect(document.body.textContent).toContain('准备写入compact');
    });

    await click(requireButtonByText('未确认'));
    await click(requireCheckboxByValue('delete'));

    await waitFor(() => {
      expect(requireCheckboxByValue('delete').checked).toBe(true);
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
      configEntries: [
        {
          key: 'max.message.bytes',
          value: '1048576',
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
      configEntries: [
        {
          key: 'retention.ms',
          value: '604800000',
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
    await click(requireButtonByText('未确认'));
    await changeValue(requireElement<HTMLInputElement>('#topic-config-requested-value'), '1000');

    await waitFor(() => {
      expect(document.body.textContent).toContain('这个 retention.ms 值明显偏短或偏长');
    });

    expect(requireButtonByText('保存修改').disabled).toBe(false);
  });

  it('shows advisory warning for suspicious but valid max.message.bytes values without triggering invalid-number state', async () => {
    const topicDetail = buildTopicDetailResponse();
    const overview: TopicOperationsOverviewResponse = {
      status: 'passed',
      message: 'ok',
      stages: [],
      configEntries: [
        {
          key: 'max.message.bytes',
          value: '1048576',
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
