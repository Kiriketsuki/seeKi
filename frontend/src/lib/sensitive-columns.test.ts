import { describe, it, expect } from 'vitest';
import { isSensitiveColumn } from './sensitive-columns';
import type { ColumnInfo } from './types';

function col(name: string): ColumnInfo {
  return {
    name,
    display_name: name,
    data_type: 'text',
    display_type: 'Text',
    is_nullable: true,
    is_primary_key: false,
  };
}

describe('isSensitiveColumn', () => {
  it('matches credential-bearing names (positive cases)', () => {
    const positives = [
      'password',
      'password_hash',
      'PasswordHash',
      'user_secret',
      'api_token',
      'access_token',
      'credential',
      'credentials',
      'private_key',
      'private-key',
      'privatekey',
      'otp',
      'otp_secret',
      '2fa',
      '2fa_enabled',
      'totp_secret',
      'mfa_secret',
      'password_salt',
    ];
    for (const name of positives) {
      expect(isSensitiveColumn(name), name).toBe(true);
    }
  });

  it('does not match ordinary column names (negative cases)', () => {
    const negatives = [
      'id',
      'name',
      'email',
      'created_at',
      'stand_id',
      'jcpl_id',
      'belt_id',
      'faults',
      'description',
      'status',
    ];
    for (const name of negatives) {
      expect(isSensitiveColumn(name), name).toBe(false);
    }
  });

  it('accepts a ColumnInfo and reads its name', () => {
    expect(isSensitiveColumn(col('password_hash'))).toBe(true);
    expect(isSensitiveColumn(col('email'))).toBe(false);
  });
});
