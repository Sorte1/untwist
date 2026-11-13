import { Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { libraryFolds } from './folds';

const known = new Set(['core-js']);

function doc(lines: string[]): Text {
  return Text.of(lines);
}

function folded(text: Text, from: number, to: number): string {
  return text.sliceString(from, to);
}

describe('the library folds', () => {
  it('hide the statement under each library comment and leave the comment and the app code shown', () => {
    const text = doc([
      'var app = 1;',
      '// core-js 3.x: internals/get-built-in.js',
      'function requireGetBuiltIn() {',
      '  if (a) {',
      '    return b;',
      '  } else {',
      '    return c;',
      '  }',
      '}',
      'app();',
    ]);
    const folds = libraryFolds(text, known, false);
    expect(folds).toHaveLength(1);
    const [fold] = folds;
    expect(fold?.lines).toBe(7);
    expect(fold?.library).toBe('core-js');
    expect(folded(text, fold?.from ?? 0, fold?.to ?? 0)).toBe(
      '\nfunction requireGetBuiltIn() {\n  if (a) {\n    return b;\n  } else {\n    return c;\n  }\n}',
    );
  });

  it('end a module property at its closing brace line, before the next property', () => {
    const text = doc(['var m = {', '  // core-js 3.x: modules/es.map.js', '  12: function (e) {', '    e();', '  },', '  13: function () {},', '};']);
    const [fold] = libraryFolds(text, known, false);
    expect(folded(text, fold?.from ?? 0, fold?.to ?? 0)).toBe('\n  12: function (e) {\n    e();\n  },');
  });

  it('fold a whole module file whose first line is the comment', () => {
    const text = doc(['// core-js 3.x: internals/own-keys.js', 'var a = 1;', 'module.exports = a;']);
    expect(libraryFolds(text, known, true)).toEqual([{ from: text.line(1).to, to: text.length, lines: 2, library: 'core-js' }]);
    expect(libraryFolds(text, known, false)).toHaveLength(1);
    expect(libraryFolds(text, known, false)[0]?.lines).toBe(1);
  });

  it('fold nothing without known libraries, for unknown comments or for a comment before a blank line', () => {
    const text = doc(['// core-js 3.x: a.js', '', 'a();', '// other 1.x: b.js', 'b();']);
    expect(libraryFolds(text, new Set(), false)).toEqual([]);
    expect(libraryFolds(text, known, false)).toEqual([]);
  });

  it('give each of two adjacent modules its own fold', () => {
    const text = doc(['// core-js 3.x: a.js', 'function a() {', '}', '// core-js 3.x: b.js', 'function b() {', '}']);
    expect(libraryFolds(text, known, false).map((fold) => fold.lines)).toEqual([2, 2]);
  });
});
