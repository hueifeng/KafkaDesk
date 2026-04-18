import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { PageFrame } from '@/components/layout/page-frame';
import { EmptyState } from '@/components/ui/empty-state';
import { TableShell } from '@/components/ui/table-shell';
import { useWorkbenchStore } from '@/app/workbench-store';
import { getAuditEvent, listAuditEvents } from '@/features/audit/api';
import type { AuditEventDetail, AuditEventSummary } from '@/features/audit/types';
import type { AppError } from '@/lib/tauri';

function getOutcomeLabel(outcome: string) {
  switch (outcome) {
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
    case 'dry_run':
      return 'Dry Run';
    case 'queued':
      return '已排队';
    case 'succeeded':
      return '成功';
    case 'failed':
      return '失败';
    default:
      return outcome;
  }
}

function prettifyJson(value?: string | null) {
  if (!value) {
    return null;
  }

  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}

export function AuditPage() {
  const activeClusterProfileId = useWorkbenchStore((state) => state.activeClusterProfileId);
  const activeClusterName = useWorkbenchStore((state) => state.activeClusterName);
  const [eventType, setEventType] = useState('');
  const [outcome, setOutcome] = useState('');
  const [startAt, setStartAt] = useState('');
  const [endAt, setEndAt] = useState('');
  const [selectedAuditId, setSelectedAuditId] = useState<string | null>(null);

  const auditListQuery = useQuery<AuditEventSummary[], AppError>({
    queryKey: ['audit-events', activeClusterProfileId, eventType, outcome, startAt, endAt],
    queryFn: () =>
      listAuditEvents({
        clusterProfileId: activeClusterProfileId || undefined,
        eventType: eventType.trim() || undefined,
        outcome: outcome || undefined,
        startAt: startAt || undefined,
        endAt: endAt || undefined,
        limit: 200,
      }),
  });

  const auditDetailQuery = useQuery<AuditEventDetail, AppError>({
    queryKey: ['audit-event', selectedAuditId],
    enabled: Boolean(selectedAuditId),
    queryFn: () => getAuditEvent(selectedAuditId!),
  });

  useEffect(() => {
    if (!auditListQuery.data?.length) {
      if (selectedAuditId) {
        setSelectedAuditId(null);
      }
      return;
    }

    if (!selectedAuditId || !auditListQuery.data.some((event) => event.id === selectedAuditId)) {
      setSelectedAuditId(auditListQuery.data[0].id);
    }
  }, [auditListQuery.data, selectedAuditId]);

  return (
    <PageFrame
      eyebrow="操作审计"
      title="审计"
      description="查看敏感操作的本地记录，确认回放请求是如何被验证、排队与记录的。"
      contextualInfo={
        <div>
          <div className="workspace-title">审计历史</div>
          <div className="workspace-note">{activeClusterProfileId ? `当前按 ${activeClusterName} 过滤` : '当前显示全部本地记录'}</div>
        </div>
      }
      actions={
        <button
          type="button"
          className="button-shell"
          data-variant="ghost"
          onClick={() => {
            setEventType('');
            setOutcome('');
            setStartAt('');
            setEndAt('');
          }}
        >
          清空筛选
        </button>
      }
    >
      <section className="workspace-surface">
        <div className="workspace-main">
          <div className="toolbar-shell mb-3" role="search" aria-label="审计筛选条件">
            <div className="lg:col-span-3">
              <label className="field-label" htmlFor="audit-event-type">事件类型</label>
              <input id="audit-event-type" className="field-shell w-full" value={eventType} onChange={(event) => setEventType(event.target.value)} placeholder="请输入事件类型，例如 replay_job_created" />
            </div>
            <div className="lg:col-span-3">
              <label className="field-label" htmlFor="audit-outcome">结果</label>
              <select id="audit-outcome" className="field-shell w-full" value={outcome} onChange={(event) => setOutcome(event.target.value)}>
                <option value="">全部</option>
                <option value="accepted">已接受</option>
                <option value="publishing">投递中</option>
                <option value="delivered">已投递</option>
                <option value="delivery_unknown">结果未知</option>
                <option value="validated">已验证</option>
                <option value="queued_local">本地排队</option>
                <option value="dry_run">Dry Run（兼容旧记录）</option>
                <option value="queued">已排队（兼容旧记录）</option>
                <option value="succeeded">成功</option>
                <option value="failed">失败</option>
              </select>
            </div>
            <div className="lg:col-span-3">
              <label className="field-label" htmlFor="audit-start-at">开始时间</label>
              <input id="audit-start-at" className="field-shell w-full" type="datetime-local" value={startAt} onChange={(event) => setStartAt(event.target.value)} />
            </div>
            <div className="lg:col-span-3">
              <label className="field-label" htmlFor="audit-end-at">结束时间</label>
              <input id="audit-end-at" className="field-shell w-full" type="datetime-local" value={endAt} onChange={(event) => setEndAt(event.target.value)} />
            </div>
          </div>

          {auditListQuery.isLoading ? (
            <div className="workspace-note py-6" role="status" aria-live="polite">正在加载审计记录…</div>
          ) : auditListQuery.isError ? (
            <EmptyState
              title="审计记录加载失败"
              description={auditListQuery.error.message}
              action={
                <button type="button" className="button-shell" data-variant="primary" onClick={() => auditListQuery.refetch()}>
                  重试
                </button>
              }
            />
          ) : (
            <TableShell
              caption="审计记录结果表，包含时间、事件类型、目标类型、摘要、结果和详情操作。"
              columns={['时间', '事件类型', '目标类型', '摘要', '结果', '操作']}
              emptyState={<EmptyState title="当前没有审计记录" description="先执行一次回放或其他敏感操作，这里会保留本地记录。" />}
            >
              {(auditListQuery.data ?? []).map((item) => (
                <tr key={item.id}>
                  <td>{item.createdAt}</td>
                  <td>{item.eventType}</td>
                  <td>{item.targetType}</td>
                  <td>{item.summary}</td>
                  <td>{getOutcomeLabel(item.outcome)}</td>
                  <td>
                    <button
                      type="button"
                      className="button-shell"
                      data-variant={selectedAuditId === item.id ? 'primary' : 'ghost'}
                      aria-pressed={selectedAuditId === item.id}
                      aria-label={`查看审计记录 ${item.eventType} 的详情`}
                      onClick={() => setSelectedAuditId(item.id)}
                    >
                      详情
                    </button>
                  </td>
                </tr>
              ))}
            </TableShell>
          )}
        </div>

        <aside className="workspace-sidebar">
          <div className="workspace-section-label">审计详情</div>
          {!selectedAuditId ? (
            <EmptyState title="尚未选择记录" description="从左侧表格中选择一条审计记录查看详情。" />
          ) : auditDetailQuery.isLoading ? (
            <div className="workspace-note py-4" role="status" aria-live="polite">正在加载审计详情…</div>
          ) : auditDetailQuery.isError ? (
            <EmptyState title="审计详情加载失败" description={auditDetailQuery.error.message} />
          ) : auditDetailQuery.data ? (
            <div className="list-stack">
              <div className="list-row">
                <div>
                  <p className="list-row-title">事件类型</p>
                  <p className="list-row-meta">{auditDetailQuery.data.eventType}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">结果</p>
                  <p className="list-row-meta">{getOutcomeLabel(auditDetailQuery.data.outcome)}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">目标</p>
                  <p className="list-row-meta">{auditDetailQuery.data.targetType}{auditDetailQuery.data.targetRef ? ` / ${auditDetailQuery.data.targetRef}` : ''}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">集群</p>
                  <p className="list-row-meta font-mono">{auditDetailQuery.data.clusterProfileId ?? '未记录'}</p>
                </div>
              </div>
              <div className="list-row">
                <div>
                  <p className="list-row-title">摘要</p>
                  <p className="list-row-meta">{auditDetailQuery.data.summary}</p>
                </div>
              </div>
              <div className="workspace-block">
                <div className="workspace-section-label">详细数据</div>
                <pre className="field-shell w-full overflow-x-auto whitespace-pre-wrap text-xs leading-6">{prettifyJson(auditDetailQuery.data.detailsJson) ?? '无附加细节'}</pre>
              </div>
            </div>
          ) : null}
        </aside>
      </section>
    </PageFrame>
  );
}
