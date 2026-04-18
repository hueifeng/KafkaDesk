export type AppPreferences = {
  preferredClusterId?: string | null;
  tableDensity: 'compact' | 'comfortable';
  defaultMessageQueryWindowMinutes: number;
  preferredTraceView: 'timeline' | 'table';
};

export type UpdateAppPreferencesInput = AppPreferences;
