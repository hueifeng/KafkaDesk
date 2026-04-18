import { describe, expect, it } from 'vitest';
import { getTableVisibilityState } from '@/components/ui/table-shell';

describe('getTableVisibilityState', () => {
  it('shows all rows when no initial limit is provided', () => {
    expect(getTableVisibilityState(24)).toEqual({
      displayedRowCount: 24,
      effectiveInitialRowCount: null,
      hiddenRowCount: 0,
    });
  });

  it('caps visible rows and reports remaining hidden rows', () => {
    expect(getTableVisibilityState(120, 50, 50)).toEqual({
      displayedRowCount: 50,
      effectiveInitialRowCount: 50,
      hiddenRowCount: 70,
    });
  });

  it('never shrinks below the configured initial row count when state drifts lower', () => {
    expect(getTableVisibilityState(120, 50, 10)).toEqual({
      displayedRowCount: 50,
      effectiveInitialRowCount: 50,
      hiddenRowCount: 70,
    });
  });
});
