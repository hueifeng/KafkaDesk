import type { ValidationStage, ValidationStageStatus } from '@/lib/tauri';

export type SchemaRegistryProfile = {
  id: string;
  name: string;
  baseUrl: string;
  authMode: 'none' | 'basic' | 'bearer';
  credentialRef?: string | null;
  notes?: string | null;
  createdAt: string;
  updatedAt: string;
};

export type SchemaRegistryProfileInput = {
  name: string;
  baseUrl: string;
  authMode: 'none' | 'basic' | 'bearer';
  credentialRef?: string | null;
  credentialSecret?: string | null;
  notes?: string | null;
};

export type SchemaRegistryProfileUpdateInput = SchemaRegistryProfileInput & {
  id: string;
};

export type SchemaRegistryConnectionTestInput = SchemaRegistryProfileInput & {
  profileId?: string | null;
};

export type SchemaRegistryConnectionTestResponse = {
  ok: boolean;
  status: Exclude<ValidationStageStatus, 'skipped'> | 'skipped';
  target: string;
  message: string;
  stages: ValidationStage[];
};

export const emptySchemaRegistryProfileInput: SchemaRegistryProfileInput = {
  name: '',
  baseUrl: '',
  authMode: 'none',
  credentialRef: '',
  credentialSecret: '',
  notes: '',
};
