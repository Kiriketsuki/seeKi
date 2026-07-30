import type { OutgoingRelationship, TableRelationships } from './types';

/**
 * Maps each column name to the outgoing FK edges it participates in. A column
 * can belong to several constraints, and every member of a composite FK maps
 * to the same edge.
 */
export function buildFkColumnMap(
  relationships: TableRelationships | null,
): Map<string, OutgoingRelationship[]> {
  const map = new Map<string, OutgoingRelationship[]>();
  if (!relationships) return map;
  for (const edge of relationships.outgoing) {
    for (const column of edge.columns) {
      const existing = map.get(column);
      if (existing) {
        existing.push(edge);
      } else {
        map.set(column, [edge]);
      }
    }
  }
  return map;
}

/** True when at least one FK edge on the column points at an allowed table. */
export function hasHoppableEdge(edges: OutgoingRelationship[] | undefined): boolean {
  return (edges ?? []).some((edge) => edge.target.allowed);
}

/**
 * The label shown on an FK column badge. With several constraints on one
 * column, the first allowed target wins, then the first target overall.
 */
export function fkBadgeTarget(
  edges: OutgoingRelationship[] | undefined,
): OutgoingRelationship | null {
  if (!edges || edges.length === 0) return null;
  return edges.find((edge) => edge.target.allowed) ?? edges[0];
}

/**
 * Reads every source-column value the edge needs to peek at its target row.
 * Returns null when any member of a composite FK is null or undefined on
 * this row, since a partial key cannot identify a unique target row.
 */
export function fkCellValues(
  edge: OutgoingRelationship,
  row: Record<string, unknown>,
): Record<string, string> | null {
  const values: Record<string, string> = {};
  for (const column of edge.columns) {
    const value = row[column];
    if (value === null || value === undefined) {
      return null;
    }
    values[column] = String(value);
  }
  return values;
}
