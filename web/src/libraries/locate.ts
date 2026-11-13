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

function scanText(text: string, file: number | null, known: ReadonlySet<string>, into: Map<string, ModuleHit[]>): void {
  let start = 0;
  let line = 1;
  while (start <= text.length) {
    const end = text.indexOf('\n', start);
    const stop = end === -1 ? text.length : end;
    const marker = text.indexOf(MARKER, start);
    if (marker !== -1 && marker < stop) {
      const comment = libraryComment(text.slice(start, stop), known);
      for (const match of comment === null ? [] : [comment, ...comment.also]) {
        if (known.has(match.library)) {
          into.set(match.library, [...(into.get(match.library) ?? []), ...hitsOf(match, file, line)]);
        }
      }
    }
    if (end === -1) {
      break;
    }
    start = end + 1;
    line += 1;
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
