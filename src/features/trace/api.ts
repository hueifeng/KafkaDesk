import { invokeCommand } from '@/lib/tauri';
import type { RunTraceQueryInput, TraceQueryResult } from '@/features/trace/types';

export function runTraceQuery(request: RunTraceQueryInput) {
  return invokeCommand<TraceQueryResult>('run_trace_query', { request });
}
