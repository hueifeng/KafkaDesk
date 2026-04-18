import { act, type ReactElement } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createRoot, type Root } from 'react-dom/client';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_CLUSTER_NAME, useWorkbenchStore } from '@/app/workbench-store';
import { AuditPage } from '@/pages/audit-page';
import { ReplayPage } from '@/pages/replay-page';
import { ReplayPolicyPage } from '@/pages/settings/replay-policy-page';
import { getAuditEvent, listAuditEvents } from '@/features/audit/api';
import { getMessageDetail } from '@/features/messages/api';
import { getReplayPolicy, updateReplayPolicy } from '@/features/replay-policy/api';
import { createReplayJob, getReplayJob, listReplayJobs } from '@/features/replay/api';
import type { AuditEventDetail, AuditEventSummary } from '@/features/audit/types';
import type { MessageDetailResponse } from '@/features/messages/types';
import type { ReplayPolicy } from '@/features/replay-policy/types';
import type { ReplayJobDetailResponse, ReplayJobSummary } from '@/features/replay/types';

vi.mock('@/features/messages/api', () => ({
  getMessageDetail: vi.fn(),
}));

vi.mock('@/features/replay-policy/api', () => ({
  getReplayPolicy: vi.fn(),
  updateReplayPolicy: vi.fn(),
}));

vi.mock('@/features/replay/api', () => ({
  createReplayJob: vi.fn(),
  getReplayJob: vi.fn(),
  listReplayJobs: vi.fn(),
}));

vi.mock('@/features/audit/api', () => ({
  getAuditEvent: vi.fn(),
  listAuditEvents: vi.fn(),
}));

const mockedGetMessageDetail = vi.mocked(getMessageDetail);
const mockedGetReplayPolicy = vi.mocked(getReplayPolicy);
const mockedUpdateReplayPolicy = vi.mocked(updateReplayPolicy);
const mockedCreateReplayJob = vi.mocked(createReplayJob);
const mockedGetReplayJob = vi.mocked(getReplayJob);
const mockedListReplayJobs = vi.mocked(listReplayJobs);
const mockedGetAuditEvent = vi.mocked(getAuditEvent);
const mockedListAuditEvents = vi.mocked(listAuditEvents);

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Array<{ root: Root; container: HTMLDivElement }> = [];

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
      mutations: {
        retry: false,
      },
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

async function changeValue(element: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement, value: string) {
  await act(async () => {
    const prototype = Object.getPrototypeOf(element);
    const descriptor = Object.getOwnPropertyDescriptor(prototype, 'value');

    descriptor?.set?.call(element, value);
    element.dispatchEvent(new Event('input', { bubbles: true }));
    element.dispatchEvent(new Event('change', { bubbles: true }));
  });
}

function makeReplayJob(overrides: Partial<ReplayJobSummary> = {}): ReplayJobSummary {
  return {
    id: 'job-1',
    status: 'delivered',
    mode: 'broker-delivery',
    targetTopic: 'sandbox.orders.retry',
    sourceTopic: 'orders',
    sourcePartition: 0,
    sourceOffset: '42',
    sourceTimestamp: '2026-04-18T09:59:00Z',
    createdAt: '2026-04-18T10:00:00Z',
    startedAt: '2026-04-18T10:00:01Z',
    completedAt: '2026-04-18T10:00:02Z',
    riskLevel: 'high',
    errorMessage: null,
    resultSummaryJson: JSON.stringify({
      note: 'Broker 已确认投递。',
      executionStage: 'delivery_confirmed',
      deliveryConfirmed: true,
    }),
    payloadEditJson: null,
    headersEditJson: null,
    keyEditJson: null,
    ...overrides,
  };
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
    if (!mounted) {
      continue;
    }

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

describe('critical workflow integration coverage', () => {
  it('submits a broker replay request after the operator completes the guarded workflow', async () => {
    const sourceMessage: MessageDetailResponse = {
      messageRef: {
        clusterProfileId: 'cluster-1',
        topic: 'orders',
        partition: 0,
        offset: '42',
      },
      timestamp: '2026-04-18T09:59:00Z',
      keyRaw: 'order-42',
      headers: [],
      payloadRaw: '{"status":"READY"}',
      payloadDecoded: '{"status":"READY"}',
      decodeStatus: 'json',
      schemaInfo: null,
      relatedHints: [],
    };

    const replayPolicy: ReplayPolicy = {
      allowLiveReplay: true,
      sandboxOnly: true,
      sandboxTopicPrefix: 'sandbox.',
      requireRiskAcknowledgement: true,
      deliveryTimeoutSeconds: 12,
      maxRetryAttempts: 2,
    };

    const submittedJob = makeReplayJob();
    const replayResponse: ReplayJobDetailResponse = {
      job: submittedJob,
      eventHistory: [
        {
          id: 'event-1',
          eventType: 'delivery_confirmed',
          createdAt: '2026-04-18T10:00:02Z',
          eventPayloadJson: '{"partition":1,"offset":99}',
        },
      ],
      auditRef: 'audit-1',
    };

    mockedGetMessageDetail.mockResolvedValue(sourceMessage);
    mockedGetReplayPolicy.mockResolvedValue(replayPolicy);
    mockedListReplayJobs.mockResolvedValueOnce([]).mockResolvedValue([submittedJob]);
    mockedCreateReplayJob.mockResolvedValue(replayResponse);
    mockedGetReplayJob.mockResolvedValue(replayResponse);

    await renderWithProviders(<ReplayPage />, '/replay?topic=orders&partition=0&offset=42');

    await waitFor(() => {
      expect(document.body.textContent).toContain('来源引用');
    });

    await changeValue(requireElement<HTMLInputElement>('#replay-target-topic'), 'sandbox.orders.retry');
    await click(requireButtonByText('切到 Broker 投递'));
    await click(requireButtonByText('确认风险'));
    await click(requireButtonByText('提交 Broker 投递回放'));

    await waitFor(() => {
      expect(mockedCreateReplayJob).toHaveBeenCalled();
      expect(mockedCreateReplayJob.mock.calls.at(-1)?.[0]).toEqual(
        expect.objectContaining({
          clusterProfileId: 'cluster-1',
          targetTopic: 'sandbox.orders.retry',
          dryRun: false,
          riskAcknowledged: true,
        }),
      );
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('审计引用：audit-1');
      expect(document.body.textContent).toContain('Broker 已确认投递。');
      expect(document.body.textContent).toContain('已收到 broker 投递确认。');
    });
  });

  it('loads audit details for the first matching record and lets operators reset filters', async () => {
    const auditEvents: AuditEventSummary[] = [
      {
        id: 'audit-1',
        createdAt: '2026-04-18 10:00',
        eventType: 'replay_job_created',
        targetType: 'replay_job',
        summary: '创建了一条回放任务',
        outcome: 'accepted',
        actorProfile: null,
        clusterProfileId: 'cluster-1',
        targetRef: 'job-1',
      },
    ];
    const auditDetail: AuditEventDetail = {
      id: 'audit-1',
      createdAt: '2026-04-18 10:00',
      eventType: 'replay_job_created',
      targetType: 'replay_job',
      targetRef: 'job-1',
      actorProfile: null,
      clusterProfileId: 'cluster-1',
      outcome: 'accepted',
      summary: '创建了一条回放任务',
      detailsJson: '{"jobId":"job-1","mode":"broker-delivery"}',
    };

    mockedListAuditEvents.mockResolvedValue(auditEvents);
    mockedGetAuditEvent.mockResolvedValue(auditDetail);

    await renderWithProviders(<AuditPage />, '/audit');

    await waitFor(() => {
      expect(mockedGetAuditEvent).toHaveBeenCalledWith('audit-1');
      expect(document.body.textContent).toContain('创建了一条回放任务');
    });

    await changeValue(requireElement<HTMLInputElement>('#audit-event-type'), 'replay_job_created');
    await changeValue(requireElement<HTMLSelectElement>('#audit-outcome'), 'accepted');

    await waitFor(() => {
      expect(
        mockedListAuditEvents.mock.calls.some(
          ([input]) =>
            input.clusterProfileId === 'cluster-1' &&
            input.eventType === 'replay_job_created' &&
            input.outcome === 'accepted' &&
            input.limit === 200,
        ),
      ).toBe(true);
    });

    await click(requireButtonByText('清空筛选'));

    await waitFor(() => {
      expect(requireElement<HTMLInputElement>('#audit-event-type').value).toBe('');
      expect(requireElement<HTMLSelectElement>('#audit-outcome').value).toBe('');
    });
  });

  it('persists replay policy edits through the settings workflow', async () => {
    const initialPolicy: ReplayPolicy = {
      allowLiveReplay: true,
      sandboxOnly: true,
      sandboxTopicPrefix: 'sandbox.',
      requireRiskAcknowledgement: true,
      deliveryTimeoutSeconds: 7,
      maxRetryAttempts: 1,
    };
    const updatedPolicy: ReplayPolicy = {
      allowLiveReplay: true,
      sandboxOnly: false,
      sandboxTopicPrefix: 'qa.',
      requireRiskAcknowledgement: true,
      deliveryTimeoutSeconds: 15,
      maxRetryAttempts: 3,
    };

    mockedGetReplayPolicy.mockResolvedValueOnce(initialPolicy).mockResolvedValue(updatedPolicy);
    mockedUpdateReplayPolicy.mockImplementation(async (input) => input);

    await renderWithProviders(<ReplayPolicyPage />, '/settings/replay-policy');

    await waitFor(() => {
      expect(requireElement<HTMLInputElement>('#replay-policy-sandbox-prefix').value).toBe('sandbox.');
    });

    await click(requireElement<HTMLButtonElement>('button[aria-label="关闭 Sandbox 主题限制"]'));
    await changeValue(requireElement<HTMLInputElement>('#replay-policy-sandbox-prefix'), 'qa.');
    await changeValue(requireElement<HTMLInputElement>('#replay-policy-delivery-timeout'), '15');
    await changeValue(requireElement<HTMLInputElement>('#replay-policy-max-retries'), '3');
    await click(requireButtonByText('保存策略'));

    await waitFor(() => {
      expect(mockedUpdateReplayPolicy).toHaveBeenCalled();
      expect(mockedUpdateReplayPolicy.mock.calls.at(-1)?.[0]).toEqual({
        allowLiveReplay: true,
        sandboxOnly: false,
        sandboxTopicPrefix: 'qa.',
        requireRiskAcknowledgement: true,
        deliveryTimeoutSeconds: 15,
        maxRetryAttempts: 3,
      });
    });

    await waitFor(() => {
      expect(document.body.textContent).toContain('回放策略已保存。');
      expect(document.body.textContent).toContain('当前不限制主题前缀。');
    });
  });
});
