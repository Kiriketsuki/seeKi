/**
 * State selection for the FK peek panel. The panel shows exactly one of these
 * states, so the precedence lives here as a pure function the tests can drive
 * without a DOM.
 */
export type PeekPanelState = 'loading' | 'error' | 'empty' | 'row';

export interface PeekPanelInput {
  loading: boolean;
  error: string | null;
  row: Record<string, unknown> | null;
}

/**
 * Loading wins over everything, because a stale error or row from the previous
 * peek must not flash while the next fetch runs. An error wins over an empty
 * result, because a failed fetch is not the same as "no linked record".
 */
export function peekPanelState({ loading, error, row }: PeekPanelInput): PeekPanelState {
  if (loading) return 'loading';
  if (error) return 'error';
  if (row === null) return 'empty';
  return 'row';
}

/** The jump footer appears only when a target table exists and a row is on screen. */
export function peekCanJump(input: PeekPanelInput, hasJumpTarget: boolean): boolean {
  return hasJumpTarget && peekPanelState(input) === 'row';
}
