import { invokeCommand } from '@/lib/tauri';
import type { CreateSavedQueryInput, SavedQuery, UpdateSavedQueryInput } from '@/features/saved-queries/types';

export function listSavedQueries() {
  return invokeCommand<SavedQuery[]>('list_saved_queries');
}

export function createSavedQuery(request: CreateSavedQueryInput) {
  return invokeCommand<SavedQuery>('create_saved_query', { request });
}

export function updateSavedQuery(request: UpdateSavedQueryInput) {
  return invokeCommand<SavedQuery>('update_saved_query', { request });
}

export function deleteSavedQuery(id: string) {
  return invokeCommand<void>('delete_saved_query', { id });
}
