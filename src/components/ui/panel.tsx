import type { PropsWithChildren, ReactNode } from 'react';

type PanelProps = PropsWithChildren<{
  title: string;
  kicker?: string;
  description?: string;
  tone?: 'signal' | 'warning';
  actions?: ReactNode;
}>;

export function Panel({ title, kicker, description, tone, actions, children }: PanelProps) {
  return (
    <section className="panel-shell" data-tone={tone}>
      <header className="panel-header">
        <div className="panel-title-group">
          {kicker ? <span className="panel-kicker">{kicker}</span> : null}
          <h2 className="panel-title">{title}</h2>
          {description ? <p className="panel-description">{description}</p> : null}
        </div>
        {actions}
      </header>
      {children}
    </section>
  );
}
