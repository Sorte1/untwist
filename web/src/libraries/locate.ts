import { libraryComment, type LibraryMatch } from './libraries';

export interface ModuleHit {
  readonly library: string;
  readonly range: string;
  readonly module: string;
  readonly file: number | null;
  readonly line: number;
}

const MARKER = '// ';

function hitsOf(match: LibraryMatch, file: number | null, line: number): ModuleHit[] {
  return match.modules.map((module) => ({ library: match.library, range: match.range, module, file, line }));
}

function newlinesBetween(text: string, from: number, to: number): number {
  let count = 0;
  for (let at = text.indexOf('\n', from); at !== -1 && at < to; at = text.indexOf('\n', at + 1)) {
    count += 1;
  }
  return count;
}

function scanText(text: string, file: number | null, known: ReadonlySet<string>, into: Map<string, ModuleHit[]>): void {
  let line = 1;
  let counted = 0;
  for (let marker = text.indexOf(MARKER); marker !== -1; ) {
    const lineStart = text.lastIndexOf('\n', marker) + 1;
    const end = text.indexOf('\n', marker);
    const stop = end === -1 ? text.length : end;
    line += newlinesBetween(text, counted, lineStart);
    counted = lineStart;
    const comment = libraryComment(text.slice(lineStart, stop), known);
    for (const match of comment === null ? [] : [comment, ...comment.also]) {
      if (known.has(match.library)) {
        into.set(match.library, [...(into.get(match.library) ?? []), ...hitsOf(match, file, line)]);
      }
    }
    marker = end === -1 ? -1 : text.indexOf(MARKER, end);
  }
}

export function locateModules(output: string, modules: readonly string[], known: ReadonlySet<string>): Map<string, ModuleHit[]> {
  const found = new Map<string, ModuleHit[]>();
  if (known.size === 0) {
    return found;
  }
  if (modules.length === 0) {
    scanText(output, null, known, found);
  } else {
    modules.forEach((text, file) => scanText(text, file, known, found));
  }
  return found;
}
