import type { ReactNode } from 'react';

type EmptyStateProps = {
  title: string;
  description: string;
  action?: ReactNode;
};

export function EmptyState({ title, description, action }: EmptyStateProps) {
  return (
    <div className="empty-state" role="alert" aria-live="polite">
      <h3 className="empty-title">{title || '暂无可用数据'}</h3>
      <p className="empty-description">{description || '暂无可显示内容，请调整筛选条件或稍后重试。'}</p>
      {action ? <div>{action}</div> : null}
    </div>
  );
}
