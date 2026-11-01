export interface Hunk {
  readonly fromA: number;
  readonly toA: number;
  readonly fromB: number;
  readonly toB: number;
}

export interface LineDiff {
  readonly hunks: readonly Hunk[];
  readonly exact: boolean;
}

export interface CharDiff {
  readonly ranges: Int32Array;
  readonly lineRanges: Int32Array;
  readonly exact: boolean;
  readonly linesA: number;
  readonly linesB: number;
  readonly longestLine: number;
}

const MYERS_MAX_EDITS = 256;
const NEWLINE = 10;

interface Range {
  readonly aLo: number;
  readonly aHi: number;
  readonly bLo: number;
  readonly bHi: number;
}

export function lineStarts(text: string): Int32Array {
  let count = 0;
  for (let at = text.indexOf('\n'); at !== -1; at = text.indexOf('\n', at + 1)) {
    count += 1;
  }
  const trailing = text.length > 0 && text.charCodeAt(text.length - 1) !== NEWLINE ? 1 : 0;
  const starts = new Int32Array(count + trailing + 1);
  let line = 1;
  for (let at = text.indexOf('\n'); at !== -1; at = text.indexOf('\n', at + 1)) {
    starts[line] = at + 1;
    line += 1;
  }
  starts[starts.length - 1] = text.length;
  return starts;
}

function intern(text: string, starts: Int32Array, ids: Map<string, number>): Int32Array {
  const out = new Int32Array(starts.length - 1);
  for (let line = 0; line < out.length; line += 1) {
    const key = text.slice(starts[line], starts[line + 1]);
    let id = ids.get(key);
    if (id === undefined) {
      id = ids.size;
      ids.set(key, id);
    }
    out[line] = id;
  }
  return out;
}

class Differ {
  readonly hunks: Hunk[] = [];
  exact = true;
  readonly #a: Int32Array;
  readonly #b: Int32Array;
  readonly #countA: Int32Array;
  readonly #countB: Int32Array;
  readonly #lastA: Int32Array;
  readonly #lastB: Int32Array;

  constructor(a: Int32Array, b: Int32Array, idCount: number) {
    this.#a = a;
    this.#b = b;
    this.#countA = new Int32Array(idCount);
    this.#countB = new Int32Array(idCount);
    this.#lastA = new Int32Array(idCount);
    this.#lastB = new Int32Array(idCount);
  }

  run(): void {
    const stack: Range[] = [{ aLo: 0, aHi: this.#a.length, bLo: 0, bHi: this.#b.length }];
    const found: Hunk[] = [];
    while (stack.length > 0) {
      const range = this.#trimmed(stack.pop() as Range);
      if (range.aLo === range.aHi || range.bLo === range.bHi) {
        if (range.aLo !== range.aHi || range.bLo !== range.bHi) {
          found.push({ fromA: range.aLo, toA: range.aHi, fromB: range.bLo, toB: range.bHi });
        }
        continue;
      }
      const anchors = this.#anchors(range);
      if (anchors.length === 0) {
        found.push(...this.#myers(range));
        continue;
      }
      let aLo = range.aLo;
      let bLo = range.bLo;
      for (const [aAt, bAt] of anchors) {
        stack.push({ aLo, aHi: aAt, bLo, bHi: bAt });
        aLo = aAt + 1;
        bLo = bAt + 1;
      }
      stack.push({ aLo, aHi: range.aHi, bLo, bHi: range.bHi });
    }
    found.sort((x, y) => x.fromA - y.fromA || x.fromB - y.fromB);
    for (const hunk of found) {
      const last = this.hunks.at(-1);
      if (last !== undefined && last.toA === hunk.fromA && last.toB === hunk.fromB) {
        this.hunks[this.hunks.length - 1] = { fromA: last.fromA, toA: hunk.toA, fromB: last.fromB, toB: hunk.toB };
      } else {
        this.hunks.push(hunk);
      }
    }
  }

  #trimmed({ aLo, aHi, bLo, bHi }: Range): Range {
    const a = this.#a;
    const b = this.#b;
    while (aLo < aHi && bLo < bHi && a[aLo] === b[bLo]) {
      aLo += 1;
      bLo += 1;
    }
    while (aHi > aLo && bHi > bLo && a[aHi - 1] === b[bHi - 1]) {
      aHi -= 1;
      bHi -= 1;
    }
    return { aLo, aHi, bLo, bHi };
  }

  #anchors({ aLo, aHi, bLo, bHi }: Range): Array<readonly [number, number]> {
    const a = this.#a;
    const b = this.#b;
    for (let at = aLo; at < aHi; at += 1) {
      const id = a[at] as number;
      this.#countA[id] = (this.#countA[id] ?? 0) + 1;
      this.#lastA[id] = at;
    }
    for (let at = bLo; at < bHi; at += 1) {
      const id = b[at] as number;
      this.#countB[id] = (this.#countB[id] ?? 0) + 1;
      this.#lastB[id] = at;
    }
    const pairs: Array<readonly [number, number]> = [];
    for (let at = aLo; at < aHi; at += 1) {
      const id = a[at] as number;
      if (this.#countA[id] === 1 && this.#countB[id] === 1) {
        pairs.push([at, this.#lastB[id] as number]);
      }
    }
    for (let at = aLo; at < aHi; at += 1) {
      this.#countA[a[at] as number] = 0;
    }
    for (let at = bLo; at < bHi; at += 1) {
      this.#countB[b[at] as number] = 0;
    }
    return increasing(pairs);
  }

  #myers(range: Range): Hunk[] {
    const script = boundedMyers(this.#a, this.#b, range, MYERS_MAX_EDITS);
    if (script === null) {
      this.exact = false;
      return [{ fromA: range.aLo, toA: range.aHi, fromB: range.bLo, toB: range.bHi }];
    }
    return script;
  }
}

function increasing(pairs: ReadonlyArray<readonly [number, number]>): Array<readonly [number, number]> {
  const tails: number[] = [];
  const previous = new Int32Array(pairs.length).fill(-1);
  for (let index = 0; index < pairs.length; index += 1) {
    const value = (pairs[index] as readonly [number, number])[1];
    let lo = 0;
    let hi = tails.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if ((pairs[tails[mid] as number] as readonly [number, number])[1] < value) {
        lo = mid + 1;
      } else {
        hi = mid;
      }
    }
    previous[index] = lo > 0 ? (tails[lo - 1] as number) : -1;
    tails[lo] = index;
  }
  const out: Array<readonly [number, number]> = [];
  for (let at = tails.at(-1) ?? -1; at !== -1; at = previous[at] as number) {
    out.push(pairs[at] as readonly [number, number]);
  }
  return out.reverse();
}

function boundedMyers(a: Int32Array, b: Int32Array, { aLo, aHi, bLo, bHi }: Range, maxEdits: number): Hunk[] | null {
  const n = aHi - aLo;
  const m = bHi - bLo;
  const limit = Math.min(maxEdits, n + m);
  const offset = limit + 1;
  const trace: Int32Array[] = [];
  let v = new Int32Array(2 * limit + 3);
  for (let d = 0; d <= limit; d += 1) {
    const next = v.slice();
    for (let k = -d; k <= d; k += 2) {
      const down = k === -d || (k !== d && (v[offset + k - 1] as number) < (v[offset + k + 1] as number));
      let x = down ? (v[offset + k + 1] as number) : (v[offset + k - 1] as number) + 1;
      let y = x - k;
      while (x < n && y < m && a[aLo + x] === b[bLo + y]) {
        x += 1;
        y += 1;
      }
      next[offset + k] = x;
      if (x >= n && y >= m) {
        trace.push(next);
        return backtrack(trace, offset, n, m, { aLo, aHi, bLo, bHi });
      }
    }
    trace.push(next);
    v = next;
  }
  return null;
}

function backtrack(trace: readonly Int32Array[], offset: number, n: number, m: number, range: Range): Hunk[] {
  const edits: Array<readonly [number, number, number, number]> = [];
  let x = n;
  let y = m;
  for (let d = trace.length - 1; d > 0; d -= 1) {
    const v = trace[d - 1] as Int32Array;
    const k = x - y;
    const down = k === -d || (k !== d && (v[offset + k - 1] as number) < (v[offset + k + 1] as number));
    const prevK = down ? k + 1 : k - 1;
    const prevX = v[offset + prevK] as number;
    const prevY = prevX - prevK;
    while (x > (down ? prevX : prevX + 1) && y > (down ? prevY + 1 : prevY)) {
      x -= 1;
      y -= 1;
    }
    edits.push(down ? [prevX, prevX, prevY, prevY + 1] : [prevX, prevX + 1, prevY, prevY]);
    x = prevX;
    y = prevY;
  }
  return edits.reverse().map(([fromA, toA, fromB, toB]) => ({
    fromA: range.aLo + fromA,
    toA: range.aLo + toA,
    fromB: range.bLo + fromB,
    toB: range.bLo + toB,
  }));
}

export function lineDiff(a: string, b: string): LineDiff {
  const startsA = lineStarts(a);
  const startsB = lineStarts(b);
  const ids = new Map<string, number>();
  const linesA = intern(a, startsA, ids);
  const linesB = intern(b, startsB, ids);
  const differ = new Differ(linesA, linesB, ids.size);
  differ.run();
  return { hunks: differ.hunks, exact: differ.exact };
}

function longest(starts: Int32Array): number {
  let most = 0;
  for (let line = 1; line < starts.length; line += 1) {
    most = Math.max(most, (starts[line] as number) - (starts[line - 1] as number));
  }
  return most;
}

export function charDiff(a: string, b: string): CharDiff {
  const startsA = lineStarts(a);
  const startsB = lineStarts(b);
  const ids = new Map<string, number>();
  const differ = new Differ(intern(a, startsA, ids), intern(b, startsB, ids), ids.size);
  ids.clear();
  differ.run();
  const ranges = new Int32Array(differ.hunks.length * 4);
  const lineRanges = new Int32Array(differ.hunks.length * 4);
  differ.hunks.forEach((hunk, index) => {
    ranges.set([startsA[hunk.fromA] as number, startsA[hunk.toA] as number, startsB[hunk.fromB] as number, startsB[hunk.toB] as number], index * 4);
    lineRanges.set([hunk.fromA, hunk.toA, hunk.fromB, hunk.toB], index * 4);
  });
  return { ranges, lineRanges, exact: differ.exact, linesA: startsA.length - 1, linesB: startsB.length - 1, longestLine: Math.max(longest(startsA), longest(startsB)) };
}
