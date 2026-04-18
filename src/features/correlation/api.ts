import { invokeCommand } from '@/lib/tauri';
import type { CorrelationRule, CorrelationRuleInput, CorrelationRuleUpdateInput } from '@/features/correlation/types';

export function listCorrelationRules() {
  return invokeCommand<CorrelationRule[]>('list_correlation_rules');
}

export function createCorrelationRule(request: CorrelationRuleInput) {
  return invokeCommand<CorrelationRule>('create_correlation_rule', { request });
}

export function updateCorrelationRule(request: CorrelationRuleUpdateInput) {
  return invokeCommand<CorrelationRule>('update_correlation_rule', { request });
}
