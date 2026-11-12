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

export function libraryFolds(doc: Text, known: ReadonlySet<string>, wholeFile: boolean): LibraryFold[] {
  if (known.size === 0) {
    return [];
  }
  const folds: LibraryFold[] = [];
  const top = wholeFile ? firstCodeLine(doc) : 0;
  let number = 1;
  while (number < doc.lines) {
    const line = doc.line(number);
    const comment = line.text.trimStart().startsWith('// ') ? libraryComment(line.text, known) : null;
    const next = doc.line(number + 1);
    if (comment === null || next.text.trim() === '') {
      number += 1;
      continue;
    }
    const end = number === top ? doc.lines : statementEnd(doc, number + 1);
    folds.push({ from: line.to, to: doc.line(end).to, lines: end - number, library: comment.library });
    number = end + 1;
  }
  return folds;
}
