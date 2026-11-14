import { reportField } from '../report-json';
import type { ModuleInfo } from './module-tree';

const NODE_MODULES = 'node_modules/';

export interface LibraryGroup {
  readonly name: string;
  readonly range: string;
  readonly order: readonly number[];
}

export interface ModuleGroups {
  readonly app: readonly number[];
  readonly libraries: readonly LibraryGroup[];
}

export function packageOf(path: string): string | null {
  if (!path.startsWith(NODE_MODULES)) {
    return null;
  }
  const parts = path.slice(NODE_MODULES.length).split('/');
  const scoped = parts[0]?.startsWith('@') === true;
  const name = scoped ? parts.slice(0, 2) : parts.slice(0, 1);
  return parts.length > name.length && name.every((part) => part !== '') ? name.join('/') : null;
}

export function libraryRanges(report: string): ReadonlyMap<string, string> {
  const rows = reportField(report, 'libraries');
  const ranges = new Map<string, string>();
  if (!Array.isArray(rows)) {
    return ranges;
  }
  for (const row of rows as readonly { library?: unknown; range?: unknown }[]) {
    if (typeof row.library === 'string' && !ranges.has(row.library)) {
      ranges.set(row.library, typeof row.range === 'string' ? row.range : '');
    }
  }
  return ranges;
}

export function groupModules(modules: readonly ModuleInfo[], order: readonly number[], ranges: ReadonlyMap<string, string>): ModuleGroups | null {
  const app: number[] = [];
  const byPackage = new Map<string, number[]>();
  for (const index of order) {
    const name = packageOf((modules[index] as ModuleInfo).path);
    if (name === null) {
      app.push(index);
    } else {
      byPackage.set(name, [...(byPackage.get(name) ?? []), index]);
    }
  }
  if (byPackage.size === 0) {
    return null;
  }
  const libraries = [...byPackage.entries()]
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([name, members]) => ({ name, range: ranges.get(name) ?? '', order: members }));
  return { app, libraries };
}

export function libraryLabel(group: LibraryGroup): string {
  return group.range === '' ? group.name : `${group.name} ${group.range}`;
}

export function countLabel(count: number): string {
  return `${count.toLocaleString('en-US')} ${count === 1 ? 'module' : 'modules'}`;
}
