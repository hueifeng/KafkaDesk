import type { PropsWithChildren } from 'react';

type BadgeTone = 'signal' | 'info' | 'success' | 'warning' | 'danger' | 'trace' | 'muted';

const toneClasses: Record<BadgeTone, string> = {
  signal: 'border-signal/35 bg-signal/10 text-signal',
  info: 'border-info/35 bg-info/10 text-info',
  success: 'border-success/35 bg-success/10 text-success',
  warning: 'border-warning/35 bg-warning/10 text-warning',
  danger: 'border-danger/35 bg-danger/10 text-danger',
  trace: 'border-trace/35 bg-trace/10 text-trace',
  muted: 'border-line bg-surface/80 text-ink-dim',
};

type BadgeProps = PropsWithChildren<{
  tone?: BadgeTone;
}>;

export function Badge({ tone = 'muted', children }: BadgeProps) {
  return <span className={`badge-shell ${toneClasses[tone]}`}>{children}</span>;
}

export function renderBadgeForStatus(status: string) {
  const statusToneMap: Record<string, BadgeTone> = {
    success: 'success',
    warning: 'warning',
    danger: 'danger',
    info: 'info',
    trace: 'trace',
    muted: 'muted',
  };

  const tone = statusToneMap[status.toLowerCase()] || 'muted';
  return <Badge tone={tone}>{status}</Badge>;
}