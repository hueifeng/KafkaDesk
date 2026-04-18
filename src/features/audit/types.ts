export type ListAuditEventsInput = {
  clusterProfileId?: string;
  eventType?: string;
  outcome?: string;
  startAt?: string;
  endAt?: string;
  limit?: number;
};

export type AuditEventSummary = {
  id: string;
  createdAt: string;
  eventType: string;
  targetType: string;
  summary: string;
  outcome: string;
  actorProfile?: string | null;
  clusterProfileId?: string | null;
  targetRef?: string | null;
};

export type AuditEventDetail = {
  id: string;
  createdAt: string;
  eventType: string;
  targetType: string;
  targetRef?: string | null;
  actorProfile?: string | null;
  clusterProfileId?: string | null;
  outcome: string;
  summary: string;
  detailsJson?: string | null;
};
