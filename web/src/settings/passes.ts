import table from './passes.json';

export interface PassRow {
  readonly name: string;
  readonly stage: string;
  readonly unsafe: boolean;
}

export const PASSES: readonly PassRow[] = table;

const ORDER = new Map(PASSES.map((pass, index) => [pass.name, index]));

export function offList(raw: string): string | undefined {
  const names = raw.split(',').filter((name) => name !== '');
  if (!names.every((name) => ORDER.has(name))) {
    return undefined;
  }
  return [...new Set(names)].sort((a, b) => (ORDER.get(a) ?? 0) - (ORDER.get(b) ?? 0)).join(',');
}

export function withPass(off: string, name: string, enabled: boolean): string {
  const names = off.split(',').filter((item) => item !== '' && item !== name);
  return offList((enabled ? names : [...names, name]).join(',')) ?? off;
}

export function stages(): ReadonlyArray<readonly [string, readonly PassRow[]]> {
  const grouped = new Map<string, PassRow[]>();
  for (const pass of PASSES) {
    grouped.set(pass.stage, [...(grouped.get(pass.stage) ?? []), pass]);
  }
  return [...grouped.entries()];
}
