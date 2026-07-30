import { describe, expect, it } from 'vitest';
import { peekCanJump, peekPanelState } from './peek-panel';

describe('peekPanelState', () => {
  it('shows the loading state while the fetch runs', () => {
    expect(peekPanelState({ loading: true, error: null, row: null })).toBe('loading');
  });

  it('keeps the loading state even when a stale row is still on screen', () => {
    expect(peekPanelState({ loading: true, error: null, row: { id: 1 } })).toBe('loading');
  });

  it('keeps the loading state even when a stale error is still set', () => {
    expect(peekPanelState({ loading: true, error: 'boom', row: null })).toBe('loading');
  });

  it('shows the error state when the fetch failed', () => {
    expect(peekPanelState({ loading: false, error: 'boom', row: null })).toBe('error');
  });

  it('prefers the error state over the empty state', () => {
    expect(peekPanelState({ loading: false, error: 'boom', row: { id: 1 } })).toBe('error');
  });

  it('shows the empty state when no row matched', () => {
    expect(peekPanelState({ loading: false, error: null, row: null })).toBe('empty');
  });

  it('shows the row state when a row arrived', () => {
    expect(peekPanelState({ loading: false, error: null, row: { id: 1 } })).toBe('row');
  });

  it('treats an empty object as a row', () => {
    expect(peekPanelState({ loading: false, error: null, row: {} })).toBe('row');
  });
});

describe('peekCanJump', () => {
  it('allows a jump when a row is on screen and the target is allowed', () => {
    expect(peekCanJump({ loading: false, error: null, row: { id: 1 } }, true)).toBe(true);
  });

  it('blocks a jump when the target sits outside the allowlist', () => {
    expect(peekCanJump({ loading: false, error: null, row: { id: 1 } }, false)).toBe(false);
  });

  it('blocks a jump while loading', () => {
    expect(peekCanJump({ loading: true, error: null, row: { id: 1 } }, true)).toBe(false);
  });

  it('blocks a jump on an error', () => {
    expect(peekCanJump({ loading: false, error: 'boom', row: null }, true)).toBe(false);
  });

  it('blocks a jump when no row matched', () => {
    expect(peekCanJump({ loading: false, error: null, row: null }, true)).toBe(false);
  });
});
