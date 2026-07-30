import type { IncomingRelationship, ReferenceEntry, TableRelationships } from './types';

/**
 * Build the `eq.` params for `/references`, from every incoming FK edge's
 * referenced columns on the current table and one row's values. A column
 * whose value on the row is null, or absent, is left out. That naturally
 * drops the whole owning edge from the server's match, since the server only
 * counts an edge whose referenced columns are all present.
 */
export function buildReferenceParams(
  relationships: TableRelationships | null,
  row: Record<string, unknown>,
): Record<string, string> {
  const params: Record<string, string> = {};
  if (!relationships) return params;

  for (const edge of relationships.incoming) {
    for (const column of edge.columns) {
      const value = row[column];
      if (value != null) {
        params[column] = String(value);
      }
    }
  }

  return params;
}

/**
 * Find the incoming edge that produced one `/references` entry, matching on
 * the source table and its FK columns in constraint order.
 */
function findIncomingEdge(
  relationships: TableRelationships | null,
  entry: ReferenceEntry,
): IncomingRelationship | undefined {
  if (!relationships) return undefined;

  return relationships.incoming.find(
    (edge) =>
      edge.source.schema === entry.schema &&
      edge.source.table === entry.table &&
      edge.source.columns.length === entry.columns.length &&
      edge.source.columns.every((column, i) => column === entry.columns[i]),
  );
}

/**
 * Zip one `/references` entry's source-side FK columns with the current
 * row's referenced values, in constraint order, for a jump to the source
 * table pre-filtered on this row. Returns null when the owning edge cannot
 * be found, or the row carries a null referenced value.
 */
export function buildJumpFilters(
  relationships: TableRelationships | null,
  row: Record<string, unknown>,
  entry: ReferenceEntry,
): Record<string, string> | null {
  const edge = findIncomingEdge(relationships, entry);
  if (!edge) return null;

  const values = edge.columns.map((column) => row[column]);
  if (values.some((value) => value == null)) return null;

  const filters: Record<string, string> = {};
  entry.columns.forEach((column, i) => {
    filters[column] = String(values[i]);
  });
  return filters;
}
