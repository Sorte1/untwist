import type { Text } from '@codemirror/state';
import { libraryComment } from './libraries';

export interface LibraryFold {
  readonly from: number;
  readonly to: number;
  readonly lines: number;
  readonly library: string;
}

const CLOSER = /^[}\])]/;

function indentOf(line: string): number {
  return line.length - line.trimStart().length;
}

function statementEnd(doc: Text, first: number): number {
  const indent = indentOf(doc.line(first).text);
  let end = first;
  for (let number = first + 1; number <= doc.lines; number += 1) {
    const line = doc.line(number).text;
    const body = line.trimStart();
    if (body === '') {
      continue;
    }
    const depth = line.length - body.length;
    if (depth > indent || (depth === indent && CLOSER.test(body))) {
      end = number;
      continue;
    }
    break;
  }
  return end;
}

function firstCodeLine(doc: Text): number {
  for (let number = 1; number <= doc.lines; number += 1) {
    if (doc.line(number).text.trim() !== '') {
      return number;
    }
  }
  return 0;
}

function commentLines(doc: Text, known: ReadonlySet<string>): Map<number, string> {
  const found = new Map<number, string>();
  let number = 0;
  for (const line of doc.iterLines()) {
    number += 1;
    if (line.includes('// ')) {
      const comment = libraryComment(line, known);
      if (comment !== null) {
        found.set(number, known.has(comment.library) ? comment.library : (comment.also.find((same) => known.has(same.library))?.library ?? comment.library));
      }
    }
  }
  return found;
}

export function libraryFolds(doc: Text, known: ReadonlySet<string>, wholeFile: boolean): LibraryFold[] {
  if (known.size === 0) {
    return [];
  }
  const folds: LibraryFold[] = [];
  const top = wholeFile ? firstCodeLine(doc) : 0;
  let after = 0;
  for (const [number, library] of commentLines(doc, known)) {
    if (number <= after || number >= doc.lines || doc.line(number + 1).text.trim() === '') {
      continue;
    }
    const end = number === top ? doc.lines : statementEnd(doc, number + 1);
    folds.push({ from: doc.line(number).to, to: doc.line(end).to, lines: end - number, library });
    after = end;
  }
  return folds;
}
