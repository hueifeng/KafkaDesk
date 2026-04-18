import { describe, expect, it } from 'vitest';
import { describeDecodeStatus, formatDecodeStatus, getDecodeStatusDefinition } from '@/features/messages/decode-status';

describe('message decode status helpers', () => {
  it('formats known statuses with operator-facing labels', () => {
    expect(formatDecodeStatus('avro-decoded')).toBe('Avro 已解码');
    expect(formatDecodeStatus('schema-registry-error')).toBe('Registry 读取失败');
  });

  it('returns truthful descriptions for supported legend entries', () => {
    expect(describeDecodeStatus('schema-auth-failed')).toContain('keyring');
    expect(getDecodeStatusDefinition('binary')?.tone).toBe('muted');
  });

  it('falls back gracefully for unknown statuses', () => {
    expect(formatDecodeStatus('custom-status')).toBe('custom-status');
    expect(describeDecodeStatus('custom-status')).toBe('当前没有可展示的解码结果');
  });
});
