export type ReplayPolicy = {
  allowLiveReplay: boolean;
  sandboxOnly: boolean;
  sandboxTopicPrefix: string;
  requireRiskAcknowledgement: boolean;
  deliveryTimeoutSeconds: number;
  maxRetryAttempts: number;
};

export type UpdateReplayPolicyInput = ReplayPolicy;
