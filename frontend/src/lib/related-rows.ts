import type { IncomingRelationship, ReferenceEntry, TableRelationships } from './types';

/**
 * Incoming FK edges whose source table is inside the connection allowlist.
 * The server never counts an edge outside the allowlist, so the UI must not
 * offer one either.
 */
export function allowedIncoming(
  relationships: TableRelationships | null,
): IncomingRelationship[] {
  if (!relationships) return [];
  return relationships.incoming.filter((edge) => edge.source.allowed);
}

/**
 * True when this table has at least one referencing table the user may open.
 * The grid uses it to decide whether the trailing "Related information" column
 * appears at all.
 */
export function hasRelatedRows(relationships: TableRelationships | null): boolean {
  return allowedIncoming(relationships).length > 0;
}

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

  for (const edge of allowedIncoming(relationships)) {
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

/**
 * Stable identity for one `/references` entry. The source table alone is not
 * unique, because the server returns one entry per incoming FK edge. A table
 * with two foreign keys to the current table, such as a sender and a recipient
 * both pointing at users, produces two entries on the same table.
 */
export function referenceEntryKey(entry: ReferenceEntry): string {
  return `${entry.schema}.${entry.table}::${entry.columns.join(',')}`;
}

/**
 * Menu label for one entry. When another entry shares the same source table,
 * the FK columns are appended so the user can tell the two edges apart.
 */
export function referenceEntryLabel(entry: ReferenceEntry, entries: ReferenceEntry[]): string {
  const sameTable = entries.filter(
    (other) => other.schema === entry.schema && other.table === entry.table,
  );
  if (sameTable.length < 2) return entry.display_name;
  return `${entry.display_name} (${entry.columns.join(', ')})`;
}
