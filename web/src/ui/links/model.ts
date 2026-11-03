export type Range = readonly [number, number];
export type Pair = readonly [number, number, number, number];

export interface OutputLink {
  readonly output: Range;
  readonly input: Range;
}

export interface InputLink {
  readonly input: Range;
  readonly outputs: readonly Range[];
}

function isPair(value: unknown): value is Pair {
  return (
    Array.isArray(value) &&
    value.length === 4 &&
    value.every((at) => Number.isInteger(at) && at >= 0) &&
    value[0] <= value[1] &&
    value[2] <= value[3]
  );
}

export function pairsOf(json: string): Pair[] {
  if (json === '') {
    return [];
  }
  try {
    const value: unknown = JSON.parse(json);
    return Array.isArray(value) ? value.filter(isPair) : [];
  } catch {
    return [];
  }
}

function innermost(pairs: readonly Pair[], at: number, side: 0 | 2): Pair | null {
  let best: Pair | null = null;
  for (const pair of pairs) {
    const start = pair[side];
    const end = pair[side + 1] as number;
    if (start <= at && at <= end && (best === null || end - start < (best[side + 1] as number) - best[side])) {
      best = pair;
    }
  }
  return best;
}

export function fromOutput(pairs: readonly Pair[], at: number): OutputLink | null {
  const pair = innermost(pairs, at, 0);
  return pair === null ? null : { output: [pair[0], pair[1]], input: [pair[2], pair[3]] };
}

export function fromInput(pairs: readonly Pair[], at: number): InputLink | null {
  const pair = innermost(pairs, at, 2);
  if (pair === null) {
    return null;
  }
  const outputs = pairs.filter((other) => other[2] === pair[2] && other[3] === pair[3]).map((other): Range => [other[0], other[1]]);
  return { input: [pair[2], pair[3]], outputs };
}

export function moduleRegion(pairs: readonly Pair[], span: Range): Range | null {
  let region: [number, number] | null = null;
  for (const pair of pairs) {
    if (pair[0] >= span[0] && pair[1] <= span[1]) {
      region = region === null ? [pair[2], pair[3]] : [Math.min(region[0], pair[2]), Math.max(region[1], pair[3])];
    }
  }
  return region;
}

export function textOffset(text: string, position: number): number {
  let offset = position;
  for (let at = text.indexOf('\r\n'); at !== -1 && at < offset; at = text.indexOf('\r\n', at + 2)) {
    offset += 1;
  }
  return offset;
}
