import { invokeCommand } from '@/lib/tauri';
import type { AppPreferences, UpdateAppPreferencesInput } from '@/features/preferences/types';

export function getAppPreferences() {
  return invokeCommand<AppPreferences>('get_app_preferences');
}

export function updateAppPreferences(request: UpdateAppPreferencesInput) {
  return invokeCommand<AppPreferences>('update_app_preferences', { request });
}
