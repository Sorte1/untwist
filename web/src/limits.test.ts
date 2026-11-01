import { describe, expect, it } from 'vitest';
import { explainLimit, firedLimits, FIXED_LIMITS } from './limits';

describe('the limits in plain words', () => {
  it('names every limit no setting lifts and says the fuel setting does not affect it', () => {
    for (const name of ['budget', 'refused', 'unpack-text', 'eval-cap', 'stack']) {
      const shown = explainLimit(name);
      expect(shown.name).not.toBe(name);
      expect(shown.why).toMatch(/not affected by the fuel setting/);
      expect(FIXED_LIMITS).toContain(name);
    }
    expect(explainLimit('eval-cap')).toEqual({ name: 'call depth cap', why: 'fixed by the browser stack, not affected by the fuel setting' });
  });

  it('keeps an unknown limit by its own name', () => {
    expect(explainLimit('new-cap')).toEqual({ name: 'new-cap', why: '' });
  });

  it('splits the fired list and the trap into explained limits, without empties', () => {
    expect(firedLimits('', null)).toEqual([]);
    expect(firedLimits('refused,eval-cap', 'stack').map((limit) => limit.name)).toEqual(['depth refusal', 'call depth cap', 'stack trap']);
  });
});
