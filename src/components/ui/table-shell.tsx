import { Children, useEffect, useMemo, useState, type PropsWithChildren, type ReactNode } from 'react';

type TableShellProps = PropsWithChildren<{
  columns: string[];
  caption?: string;
  emptyState?: ReactNode;
  initialVisibleRowCount?: number;
  rowLabel?: string;
}>;

export function getTableVisibilityState(totalRows: number, initialVisibleRowCount?: number | null, visibleRowCount?: number | null) {
  const effectiveInitialRowCount = typeof initialVisibleRowCount === 'number' && initialVisibleRowCount > 0 ? initialVisibleRowCount : null;

  if (effectiveInitialRowCount === null) {
    return {
      displayedRowCount: totalRows,
      effectiveInitialRowCount,
      hiddenRowCount: 0,
    };
  }

  const displayedRowCount = typeof visibleRowCount === 'number'
    ? Math.max(effectiveInitialRowCount, Math.min(visibleRowCount, totalRows))
    : Math.min(effectiveInitialRowCount, totalRows);

  return {
    displayedRowCount,
    effectiveInitialRowCount,
    hiddenRowCount: Math.max(totalRows - displayedRowCount, 0),
  };
}

export function TableShell({ caption, columns, emptyState, initialVisibleRowCount, rowLabel = '条记录', children }: TableShellProps) {
  const rows = useMemo(() => Children.toArray(children), [children]);
  const totalRows = rows.length;
  const [visibleRowCount, setVisibleRowCount] = useState<number | null>(initialVisibleRowCount ?? null);

  useEffect(() => {
    if (typeof initialVisibleRowCount !== 'number' || initialVisibleRowCount <= 0) {
      setVisibleRowCount(null);
      return;
    }

    setVisibleRowCount((current) => {
      if (typeof current !== 'number') {
        return initialVisibleRowCount;
      }

      if (current < initialVisibleRowCount) {
        return initialVisibleRowCount;
      }

      return Math.min(current, totalRows);
    });
  }, [initialVisibleRowCount, totalRows]);

  const visibility = getTableVisibilityState(totalRows, initialVisibleRowCount, visibleRowCount);
  const displayedRows = visibility.effectiveInitialRowCount === null ? rows : rows.slice(0, visibility.displayedRowCount);
  const canShowMore = visibility.hiddenRowCount > 0 && visibility.effectiveInitialRowCount !== null;
  const nextVisibleRowCount = visibility.effectiveInitialRowCount === null
    ? totalRows
    : Math.min(totalRows, visibility.displayedRowCount + visibility.effectiveInitialRowCount);

  return (
    <div className="table-shell">
      <div className="table-scroll">
        <table>
          {caption ? <caption className="sr-only">{caption}</caption> : null}
          <thead>
            <tr>
              {columns.map((column) => (
                <th key={column} scope="col">
                  {column}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {totalRows > 0 ? (
              displayedRows
            ) : (
              <tr>
                <td colSpan={columns.length} className="p-0" role="alert" aria-live="polite">
                  {emptyState}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {canShowMore ? (
        <div className="table-shell-footer">
          <p className="table-shell-summary">
            当前显示 {visibility.displayedRowCount} / {totalRows} {rowLabel}
          </p>
          <div className="table-shell-actions">
            <button
              type="button"
              className="button-shell"
              data-variant="ghost"
              onClick={() => setVisibleRowCount(nextVisibleRowCount)}
            >
              再显示 {nextVisibleRowCount - visibility.displayedRowCount} {rowLabel}
            </button>
            <button
              type="button"
              className="button-shell"
              data-variant="ghost"
              onClick={() => setVisibleRowCount(totalRows)}
            >
              显示全部
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
