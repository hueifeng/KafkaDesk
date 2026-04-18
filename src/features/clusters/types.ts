import type { ValidationStage, ValidationStageStatus } from '@/lib/tauri';

export type ClusterProfileSummary = {
  id: string;
  name: string;
  environment: string;
  bootstrapServers: string;
  authMode: string;
  authCredentialRef?: string | null;
  tlsMode: string;
  tlsCaCertPath?: string | null;
  tlsClientCertPath?: string | null;
  schemaRegistryProfileId?: string | null;
  isFavorite: boolean;
  lastConnectedAt?: string | null;
};

export type ClusterProfile = {
  id: string;
  name: string;
  environment: string;
  bootstrapServers: string;
  authMode: string;
  authCredentialRef?: string | null;
  tlsMode: string;
  tlsCaCertPath?: string | null;
  tlsClientCertPath?: string | null;
  tlsClientKeyPath?: string | null;
  schemaRegistryProfileId?: string | null;
  notes?: string | null;
  tags: string[];
  isFavorite: boolean;
  createdAt: string;
  updatedAt: string;
  lastConnectedAt?: string | null;
  isArchived: boolean;
};

export type ClusterConnectionTestResponse = {
  ok: boolean;
  status: Exclude<ValidationStageStatus, 'skipped'> | 'skipped';
  attemptedBrokers: number;
  reachableBrokers: number;
  message: string;
  stages: ValidationStage[];
};

export type ClusterConnectionTestInput = ClusterProfileInput & {
  profileId?: string | null;
};

export type ClusterProfileInput = {
  name: string;
  environment: string;
  bootstrapServers: string;
  authMode: string;
  authCredentialRef?: string | null;
  authSecret?: string | null;
  tlsMode: string;
  tlsCaCertPath?: string | null;
  tlsClientCertPath?: string | null;
  tlsClientKeyPath?: string | null;
  schemaRegistryProfileId?: string | null;
  notes?: string | null;
  tags: string[];
};

export type ClusterProfileUpdateInput = ClusterProfileInput & {
  id: string;
  isFavorite: boolean;
  isArchived: boolean;
};

export const emptyClusterProfileInput: ClusterProfileInput = {
  name: '',
  environment: 'dev',
  bootstrapServers: '',
  authMode: 'none',
  authCredentialRef: '',
  authSecret: '',
  tlsMode: 'system-default',
  tlsCaCertPath: '',
  tlsClientCertPath: '',
  tlsClientKeyPath: '',
  schemaRegistryProfileId: null,
  notes: '',
  tags: [],
};
