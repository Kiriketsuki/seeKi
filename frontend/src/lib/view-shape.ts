// Pure helpers for building a `ViewDefinitionShape`. Extracted from
// ColumnPicker.svelte so a shape can be assembled outside a component, for
// example the transient shape the inline related-columns picker builds on
// the fly (see RelatedColumnsPicker.svelte and App.svelte).

import type {
  ColumnInfo,
  PickedRelatedColumn,
  ViewColumn,
  ViewDefinitionShape,
  ViewSourceRef,
} from './types';
import { RELATED_COLUMNS_KEY_PREFIX } from './constants';

export type SourceMode = 'base' | 'fk' | 'match' | 'self';

export interface BuildSourceDefinitionParams {
  mode: SourceMode;
  baseSchema: string;
  baseTable: string;
  selectedSchema: string;
  selectedTable: string;
  activeSourceId: string | null;
  selfEntityColumn: string;
  selfOrderColumn: string;
  selfDirection: 'previous' | 'next';
  selectedBaseMatchColumn: string;
  selectedSourceMatchColumn: string;
}

/**
 * Build the `ViewSourceRef` a column picker step implies. Returns null for
 * the base table, since the base table is never listed as a joined source.
 */
export function buildSourceDefinition(
  params: BuildSourceDefinitionParams,
): ViewSourceRef | null {
  const {
    mode,
    baseSchema,
    baseTable,
    selectedSchema,
    selectedTable,
    activeSourceId,
    selfEntityColumn,
    selfOrderColumn,
    selfDirection,
    selectedBaseMatchColumn,
    selectedSourceMatchColumn,
  } = params;

  if (mode === 'base') return null;

  if (mode === 'self') {
    return {
      id: activeSourceId ?? `self-${selfDirection}`,
      kind: 'self',
      schema: baseSchema,
      table: baseTable,
      label:
        selfDirection === 'previous'
          ? 'This table again (previous row)'
          : 'This table again (next row)',
      self: {
        entity_column: selfEntityColumn,
        order_column: selfOrderColumn,
        direction: selfDirection,
      },
    };
  }

  if (mode === 'match') {
    return {
      id: activeSourceId ?? `match-${selectedSchema}.${selectedTable}`,
      kind: 'match',
      schema: selectedSchema,
      table: selectedTable,
      label: `Match ${selectedSchema}.${selectedTable}`,
      match: {
        base_column: selectedBaseMatchColumn,
        source_column: selectedSourceMatchColumn,
      },
    };
  }

  return {
    id: activeSourceId ?? `fk-${selectedSchema}.${selectedTable}`,
    kind: 'fk',
    schema: selectedSchema,
    table: selectedTable,
    label: `${selectedSchema}.${selectedTable}`,
  };
}

/** Deterministic source id for an FK hop, shared with fk-columns.ts naming. */
export function fkSourceId(schema: string, table: string): string {
  return `fk-${schema}.${table}`;
}

/**
 * Sanitize a generated identifier the same way the backend's
 * sanitize_generated_identifier does: keep alphanumerics, underscore, hyphen
 * and space, replace every other character, and fall back to "source" when
 * nothing readable survives.
 */
export function sanitizeGeneratedIdentifier(value: string): string {
  const sanitized = Array.from(value)
    .map((ch) => (/[\p{L}\p{N}_\- ]/u.test(ch) ? ch : '_'))
    .join('');
  return sanitized.trim().length === 0 ? 'source' : sanitized;
}

/**
 * Prefix the backend puts in front of a colliding column name. The backend's
 * output_name_prefix_for_saved_column prefers the sanitized source_id and
 * falls back to the source table, so a related column joined through source
 * id `fk-public.users` resolves to `fk-public_users__name`, not
 * `users__name`. The frontend must agree, since it maps output names back to
 * their "from {table}" header label.
 */
function outputNamePrefix(column: ViewColumn): string {
  const fromSourceId = column.source_id ? sanitizeGeneratedIdentifier(column.source_id) : '';
  return fromSourceId.length > 0 ? fromSourceId : column.source_table;
}

/**
 * Resolve the output column name for each plain source column, the same way
 * the backend's resolve_saved_view_output_names does for the no-alias,
 * no-aggregate, no-derived case: a bare column name stays bare unless it
 * collides with another selected column, in which case it gets a
 * `{prefix}__{column}` prefix. buildRelatedShape never sets alias, aggregate,
 * or derived, so this covers every shape it produces.
 */
export function resolveViewColumnOutputNames(columns: ViewColumn[]): string[] {
  const counts = new Map<string, number>();
  for (const column of columns) {
    counts.set(column.column_name, (counts.get(column.column_name) ?? 0) + 1);
  }
  return columns.map((column) =>
    (counts.get(column.column_name) ?? 0) > 1
      ? `${outputNamePrefix(column)}__${column.column_name}`
      : column.column_name,
  );
}

/**
 * Build a transient view shape that projects every base-table column plus
 * the picked related columns, each joined through its outgoing FK. The
 * shape carries no grouping or ranking, only a plain one-hop-per-far-table
 * join list, so it stays a straight denormalizing projection.
 */
export function buildRelatedShape(
  baseSchema: string,
  baseTable: string,
  baseColumns: ColumnInfo[],
  picked: PickedRelatedColumn[],
): ViewDefinitionShape {
  const baseSourceColumns: ViewColumn[] = baseColumns.map((column) => ({
    source_schema: baseSchema,
    source_table: baseTable,
    column_name: column.name,
  }));

  const sources = new Map<string, ViewSourceRef>();
  const relatedSourceColumns: ViewColumn[] = picked.map((related) => {
    const sourceId = fkSourceId(related.schema, related.table);
    if (!sources.has(sourceId)) {
      sources.set(sourceId, {
        id: sourceId,
        kind: 'fk',
        schema: related.schema,
        table: related.table,
        label: related.tableDisplayName,
      });
    }
    return {
      source_id: sourceId,
      source_schema: related.schema,
      source_table: related.table,
      column_name: related.column,
    };
  });

  return {
    columns: [...baseSourceColumns, ...relatedSourceColumns],
    filters: {},
    sources: Array.from(sources.values()),
  };
}

/** localStorage key for a table's picked inline related columns. */
export function relatedColumnsStorageKey(schema: string, table: string): string {
  return `${RELATED_COLUMNS_KEY_PREFIX}${schema}.${table}`;
}

/**
 * Read the picked related columns for a table from localStorage. Returns an
 * empty array on missing, malformed, or non-array stored state, so a picker
 * a future release removes never crashes the table load.
 */
export function loadRelatedColumns(schema: string, table: string): PickedRelatedColumn[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem(relatedColumnsStorageKey(schema, table));
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as PickedRelatedColumn[]) : [];
  } catch {
    return [];
  }
}

/**
 * Persist the picked related columns for a table. An empty list removes the
 * stored key entirely, so an empty array and "never picked anything" read
 * back identically.
 */
export function saveRelatedColumns(
  schema: string,
  table: string,
  next: PickedRelatedColumn[],
): void {
  if (typeof localStorage === 'undefined') return;
  try {
    if (next.length === 0) {
      localStorage.removeItem(relatedColumnsStorageKey(schema, table));
    } else {
      localStorage.setItem(relatedColumnsStorageKey(schema, table), JSON.stringify(next));
    }
  } catch {
    // Non-fatal — related-column persistence should never break the UI.
  }
}
