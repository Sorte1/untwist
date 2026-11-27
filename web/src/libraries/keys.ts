export interface KeyedTable {
  readonly id: string;
  readonly keys: readonly string[];
}

export function distinctKeys(tables: readonly KeyedTable[], generic: ReadonlySet<string>, shared = 3): KeyedTable[] {
  const counts = new Map<string, number>();
  for (const table of tables) {
    for (const key of new Set(table.keys)) {
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
  }
  return tables.map((table) => ({ id: table.id, keys: [...new Set(table.keys)].filter((key) => !generic.has(key) && (counts.get(key) ?? 0) < shared) }));
}
