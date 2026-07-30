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
 * The plain-language badge for an FK reachability hop count. One hop means
 * the table sits behind a single foreign key, more than one means the path
 * runs through at least one other table. Returns an empty string when the
 * hop count is missing or not positive, so the caller can skip the badge.
 */
export function hopBadgeLabel(hops: number | undefined): string {
  if (hops === 1) return 'Directly linked';
  if (hops !== undefined && hops > 1) return 'Linked through another table';
  return '';
}
