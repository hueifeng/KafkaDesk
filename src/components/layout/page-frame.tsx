import type { ReactNode } from 'react';
import { Badge } from '@/components/ui/badge';

type PageTag = {
  label: string;
  tone?: 'signal' | 'info' | 'success' | 'warning' | 'danger' | 'trace' | 'muted';
};

type PageFrameProps = {
  contextualInfo?: ReactNode;
  eyebrow: string;
  title: string;
  description: string;
  tags?: PageTag[];
  actions?: ReactNode;
  summary?: ReactNode;
  aside?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
};

export function PageFrame({
  contextualInfo,
  eyebrow,
  title,
  description,
  tags,
  actions,
  summary,
  aside,
  footer,
  children,
}: PageFrameProps) {
  return (
    <section className="page-frame">
      <header className="page-header">
        <div className="page-header-copy">
          <span className="page-eyebrow">{eyebrow}</span>
          <div>
            <h1 className="page-title">{title}</h1>
            <p className="page-description text-balance">{description}</p>
          </div>
          {tags?.length ? (
            <div className="page-tags">
              {tags.map((tag) => (
                <Badge key={tag.label} tone={tag.tone}>
                  {tag.label}
                </Badge>
              ))}
            </div>
          ) : null}
        </div>
        {actions}
      </header>

      {contextualInfo ? (<div className="page-contextual-info">{contextualInfo}</div>) : null}
      {summary ? <div className="page-summary-strip">{summary}</div> : null}

      <div className={aside ? 'page-grid' : 'page-main'}>
        <div className="page-main">{children}</div>
        {aside ? <aside className="page-aside">{aside}</aside> : null}
      </div>

      {footer}
    </section>
  );
}
