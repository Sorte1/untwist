import { describe, expect, it } from 'vitest';
import { lineDiff, lineStarts, type LineDiff } from './lines';

function apply(a: string, b: string, result: LineDiff): string {
  const startsA = lineStarts(a);
  const startsB = lineStarts(b);
  let out = '';
  let at = 0;
  for (const hunk of result.hunks) {
    out += a.slice(startsA[at], startsA[hunk.fromA]);
    out += b.slice(startsB[hunk.fromB], startsB[hunk.toB]);
    at = hunk.toA;
  }
  return out + a.slice(startsA[at]);
}

function lines(count: number, tag = ''): string {
  return Array.from({ length: count }, (_, index) => `line ${index}${tag};\n`).join('');
}

function seeded(seed: number): () => number {
  let state = seed;
  return () => {
    state = (state * 1103515245 + 12345) & 0x7fffffff;
    return state / 0x7fffffff;
  };
}

describe('the line diff for large outputs', () => {
  it('finds nothing between equal texts', () => {
    const text = lines(1000);
    expect(lineDiff(text, text).hunks).toEqual([]);
  });

  it('reports one hunk per changed line, in line numbers', () => {
    const a = lines(10);
    const b = a.replace('line 3;', 'line three;').replace('line 7;\n', '');
    expect(lineDiff(a, b).hunks).toEqual([
      { fromA: 3, toA: 4, fromB: 3, toB: 4 },
      { fromA: 7, toA: 8, fromB: 7, toB: 7 },
    ]);
  });

  it('handles insertions at the ends, a missing final newline and empty sides', () => {
    expect(lineDiff('', 'a\n').hunks).toEqual([{ fromA: 0, toA: 0, fromB: 0, toB: 1 }]);
    expect(lineDiff('a\n', '').hunks).toEqual([{ fromA: 0, toA: 1, fromB: 0, toB: 0 }]);
    expect(lineDiff('a\nb', 'a\nb\n').hunks.length).toBe(1);
    expect(lineDiff('x\na\n', 'a\ny\n').hunks).toEqual([
      { fromA: 0, toA: 1, fromB: 0, toB: 0 },
      { fromA: 2, toA: 2, fromB: 1, toB: 2 },
    ]);
  });

  it('rebuilds B from A and the hunks on random edits of repeated lines', () => {
    const random = seeded(7);
    const pool = ['{\n', '}\n', 'return a;\n', 'x++;\n', 'var b = 1;\n', '\n'];
    for (let round = 0; round < 200; round += 1) {
      const a = Array.from({ length: 40 + Math.floor(random() * 80) }, () => pool[Math.floor(random() * pool.length)]).join('');
      const words = a.split(/(?<=\n)/);
      const b = words
        .flatMap((word) => {
          const roll = random();
          return roll < 0.08 ? [] : roll < 0.16 ? [word, pool[Math.floor(random() * pool.length)] ?? ''] : roll < 0.2 ? ['changed;\n'] : [word];
        })
        .join('');
      expect(apply(a, b, lineDiff(a, b))).toBe(b);
    }
  });

  it('keeps a few scattered edits small on a 300,000-line text', () => {
    const a = lines(300_000);
    const b = a.replace('line 1000;', 'line 1000 changed;').replace('line 150000;\n', '').replace('line 299999;', 'line 299999;\nextra;');
    const result = lineDiff(a, b);
    expect(result.hunks).toHaveLength(3);
    expect(result.exact).toBe(true);
    expect(apply(a, b, result)).toBe(b);
  });

  it('gives up on detail, not on correctness, when two ranges share no anchors and differ a lot', () => {
    const a = lines(3000, 'a').replace(/;/g, ';{}');
    const b = lines(3000, 'b');
    const result = lineDiff(a, b);
    expect(apply(a, b, result)).toBe(b);
    expect(result.hunks.length).toBeGreaterThan(0);
  });
});
