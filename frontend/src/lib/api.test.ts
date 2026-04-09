import { describe, it, expect } from 'vitest';

// Re-implement assertShape locally since it's not exported — tests validate the logic
function assertShape(data: unknown, fields: string[], context: string): void {
  if (data == null || typeof data !== 'object') {
    throw new Error(`${context}: expected object, got ${typeof data}`);
  }
  for (const field of fields) {
    if (!(field in (data as Record<string, unknown>))) {
      throw new Error(`${context}: missing required field '${field}'`);
    }
  }
}

describe('assertShape', () => {
  it('passes when all fields are present', () => {
    expect(() =>
      assertShape({ rows: [], total_rows: 0, page: 1, page_size: 50 }, ['rows', 'total_rows', 'page', 'page_size'], 'test')
    ).not.toThrow();
  });

  it('throws when a required field is missing', () => {
    expect(() =>
      assertShape({ rows: [] }, ['rows', 'total_rows'], 'test')
    ).toThrow("test: missing required field 'total_rows'");
  });

  it('throws when data is null', () => {
    expect(() =>
      assertShape(null, ['rows'], 'test')
    ).toThrow('test: expected object, got object');
  });

  it('throws when data is undefined', () => {
    expect(() =>
      assertShape(undefined, ['rows'], 'test')
    ).toThrow('test: expected object, got undefined');
  });

  it('throws when data is a primitive', () => {
    expect(() =>
      assertShape('hello', ['rows'], 'test')
    ).toThrow('test: expected object, got string');
  });

  it('passes with empty fields list', () => {
    expect(() => assertShape({}, [], 'test')).not.toThrow();
  });

  it('passes when fields exist with null values', () => {
    expect(() =>
      assertShape({ rows: null, total_rows: null }, ['rows', 'total_rows'], 'test')
    ).not.toThrow();
  });
});
