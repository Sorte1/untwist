export interface ExplainedLimit {
  readonly name: string;
  readonly why: string;
}

const UNAFFECTED = 'not affected by the fuel setting';

const WORDS: Readonly<Record<string, ExplainedLimit>> = {
  budget: { name: 'nesting budget', why: `the browser stack sets how deeply code may nest; ${UNAFFECTED}` },
  refused: { name: 'depth refusal', why: `code nested deeper than the browser stack allows was left as it was; ${UNAFFECTED}` },
  'unpack-text': {
    name: 'unpack text depth',
    why: `the result nests too deeply to parse again for unpacking, so it is written without a bundle; fixed by the browser stack, ${UNAFFECTED}`,
  },
  'eval-cap': { name: 'call depth cap', why: `fixed by the browser stack, ${UNAFFECTED}` },
  stack: { name: 'stack trap', why: `the browser stack ran out, so the run was redone with the deepest code refused; ${UNAFFECTED}` },
};

export const PARTIAL_LIMITS: readonly string[] = ['refused'];

export const FIXED_LIMITS: readonly string[] = Object.keys(WORDS);

export function explainLimit(name: string): ExplainedLimit {
  return Object.hasOwn(WORDS, name) ? (WORDS[name] as ExplainedLimit) : { name, why: '' };
}

export function firedLimits(limits: string, trap: string | null): ExplainedLimit[] {
  return [...limits.split(','), trap ?? ''].filter((part) => part !== '').map(explainLimit);
}

export function explainedLine(limit: ExplainedLimit): string {
  return limit.why === '' ? limit.name : `${limit.name} — ${limit.why}`;
}
