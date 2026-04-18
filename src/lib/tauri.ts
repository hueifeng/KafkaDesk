import { invoke } from '@tauri-apps/api/core';

export type AppErrorCategory =
  | 'validation_error'
  | 'config_error'
  | 'connectivity_error'
  | 'auth_error'
  | 'tls_error'
  | 'timeout_error'
  | 'unsupported_feature'
  | 'internal_error';

export type ValidationStageStatus = 'passed' | 'warning' | 'failed' | 'skipped';

export type ValidationStage = {
  key: string;
  label: string;
  status: ValidationStageStatus;
  message: string;
  detail?: string;
  errorCategory?: AppErrorCategory;
  retriable?: boolean;
};

export type AppError = {
  category: AppErrorCategory;
  code: string;
  message: string;
  details?: Record<string, unknown>;
  retriable?: boolean;
};

function isAppError(value: unknown): value is AppError {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as { code?: unknown }).code === 'string' &&
    typeof (value as { message?: unknown }).message === 'string'
  );
}

export function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }

  return '__TAURI_INTERNALS__' in window;
}

export async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw {
      category: 'internal_error',
      code: 'runtime.unavailable',
      message:
        'KafkaDesk local runtime is unavailable. Run this route inside the Tauri shell to use local commands.',
      retriable: false,
    } satisfies AppError;
  }

  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isAppError(error)) {
      throw error;
    }

    throw {
      category: 'internal_error',
      code: 'internal.unexpected',
      message: error instanceof Error ? error.message : 'Unexpected runtime failure.',
      retriable: false,
    } satisfies AppError;
  }
}
