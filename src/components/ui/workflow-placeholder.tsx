import { Link } from 'react-router-dom';
import { EmptyState } from '@/components/ui/empty-state';

type PlaceholderAction = {
  label: string;
  to: string;
  variant?: 'primary' | 'ghost';
};

type WorkflowPlaceholderProps = {
  title: string;
  kicker?: string;
  description: string;
  emptyTitle: string;
  emptyDescription: string;
  actions?: PlaceholderAction[];
};

export function WorkflowPlaceholder({
  title,
  kicker,
  description,
  emptyTitle,
  emptyDescription,
  actions = [],
}: WorkflowPlaceholderProps) {
  return (
    <section className="placeholder-surface">
      <div className="placeholder-head">
        {kicker ? <div className="panel-kicker">{kicker}</div> : null}
        <h2 className="placeholder-title">{title}</h2>
        <p className="placeholder-description">{description}</p>
      </div>
      <EmptyState
        title={emptyTitle}
        description={emptyDescription}
        action={
          actions.length ? (
            <div className="placeholder-actions">
              {actions.map((action) => (
                <Link
                  key={`${action.to}-${action.label}`}
                  to={action.to}
                  className="button-shell"
                  data-variant={action.variant ?? 'primary'}
                >
                  {action.label}
                </Link>
              ))}
            </div>
          ) : null
        }
      />
    </section>
  );
}
