import type { ColumnInfo } from './types';

// Columns whose names match credential/secret-bearing patterns are masked in the
// grid by default and revealed per-cell on demand. The match is on the column
// NAME only (never the value) so the predicate stays pure and side-effect free.
const SENSITIVE_NAME_PATTERN =
  /(password|secret|token|hash|credential|private[_-]?key|otp|2fa|totp|mfa|salt)/i;

// Accept either a ColumnInfo or a bare column name so callers in the grid and
// unit tests can both use it without constructing a full ColumnInfo.
export function isSensitiveColumn(column: ColumnInfo | string): boolean {
  const name = typeof column === 'string' ? column : column.name;
  return SENSITIVE_NAME_PATTERN.test(name);
}
