import { describe, expect, it } from 'vitest';
import { BANNER_TEXT, undecodedInSafe } from './strings-banner';

const report = (...notes: Array<readonly [string, string]>) =>
  JSON.stringify({ notes: notes.map(([pass, message]) => ({ level: 'warning', pass, kind: 'refusal', message })) });

const KEPT = (n: number) => [
  'remove-decoder-machinery',
  `decoder machinery kept: ${n} reference(s) to the string array or its decoders did not decode`,
] as const;

describe('the strings-not-decoded banner', () => {
  it('counts the decoder calls a Safe run left in place, the largest unit first', () => {
    expect(undecodedInSafe(report(KEPT(108_148), KEPT(108_072)), '')).toBe(108_148);
    expect(undecodedInSafe(report(KEPT(2)), '--rename simple')).toBe(2);
  });

  it('stays quiet for an Unsafe run, a run with nothing left, other kept-machinery reasons and bad reports', () => {
    expect(undecodedInSafe(report(KEPT(5)), '--unsafe')).toBeNull();
    expect(undecodedInSafe(report(['remove-decoder-machinery', 'decoder machinery kept: a machinery statement has no usable span']), '')).toBeNull();
    expect(undecodedInSafe(report(['find-string-array', 'string array found: 3 entries']), '')).toBeNull();
    expect(undecodedInSafe(report(), '')).toBeNull();
    expect(undecodedInSafe('not json', '')).toBeNull();
    expect(undecodedInSafe(report(KEPT(5)), null)).toBeNull();
  });

  it('says what happened and what Unsafe does', () => {
    expect(BANNER_TEXT).toBe('Strings were not decoded in Safe mode. Unsafe mode decodes them.');
  });
});
