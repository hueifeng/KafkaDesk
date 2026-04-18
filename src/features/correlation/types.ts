export type CorrelationStrategy = 'header-match' | 'key-match' | 'decoded-field-match' | 'ordered-multi-topic';

export type CorrelationRule = {
  id: string;
  name: string;
  clusterProfileId: string;
  isEnabled: boolean;
  matchStrategy: CorrelationStrategy;
  scopeJson: string;
  ruleJson: string;
  createdAt: string;
  updatedAt: string;
};

export type CorrelationRuleInput = {
  name: string;
  clusterProfileId: string;
  isEnabled: boolean;
  matchStrategy: CorrelationStrategy;
  scopeJson: string;
  ruleJson: string;
};

export type CorrelationRuleUpdateInput = CorrelationRuleInput & {
  id: string;
};

export const emptyCorrelationRuleInput: CorrelationRuleInput = {
  name: '',
  clusterProfileId: '',
  isEnabled: true,
  matchStrategy: 'header-match',
  scopeJson: '{\n  "topics": [],\n  "headers": []\n}',
  ruleJson: '{\n  "matchKey": "traceId",\n  "mode": "exact"\n}',
};
