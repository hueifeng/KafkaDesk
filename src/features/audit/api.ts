import { invokeCommand } from '@/lib/tauri';
import type { AuditEventDetail, AuditEventSummary, ListAuditEventsInput } from '@/features/audit/types';

export function listAuditEvents(request: ListAuditEventsInput) {
  return invokeCommand<AuditEventSummary[]>('list_audit_events', { request });
}

export function getAuditEvent(id: string) {
  return invokeCommand<AuditEventDetail>('get_audit_event', { id });
}
