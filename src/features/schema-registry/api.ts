import { invokeCommand } from '@/lib/tauri';
import type {
  SchemaRegistryConnectionTestInput,
  SchemaRegistryConnectionTestResponse,
  SchemaRegistryProfile,
  SchemaRegistryProfileInput,
  SchemaRegistryProfileUpdateInput,
} from '@/features/schema-registry/types';

export function listSchemaRegistryProfiles() {
  return invokeCommand<SchemaRegistryProfile[]>('list_schema_registry_profiles');
}

export function createSchemaRegistryProfile(request: SchemaRegistryProfileInput) {
  return invokeCommand<SchemaRegistryProfile>('create_schema_registry_profile', { request });
}

export function updateSchemaRegistryProfile(request: SchemaRegistryProfileUpdateInput) {
  return invokeCommand<SchemaRegistryProfile>('update_schema_registry_profile', { request });
}

export function testSchemaRegistryProfile(request: SchemaRegistryConnectionTestInput) {
  return invokeCommand<SchemaRegistryConnectionTestResponse>('test_schema_registry_profile', { request });
}
