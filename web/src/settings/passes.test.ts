import { describe, expect, it } from 'vitest';
import { flagsFrom } from './flags';
import { parseHash, writeHash } from './hash';
import { offList, PASSES, stages, withPass } from './passes';
import { CLI_DEFAULTS, DEFAULTS } from './settings';

const index = (name: string) => PASSES.findIndex((pass) => pass.name === name);

describe('passes', () => {
  it('lists every engine pass once, each under a stage', () => {
    expect(PASSES.length).toBeGreaterThan(50);
    expect(new Set(PASSES.map((pass) => pass.name)).size).toBe(PASSES.length);
    expect(stages().flatMap(([, rows]) => rows).length).toBe(PASSES.length);
    expect(PASSES.filter((pass) => pass.unsafe).map((pass) => pass.name)).toContain('template-literals');
  });

  it('keeps the off list in table order without repeats and refuses an unknown name', () => {
    const [first, second] = [index('yoda') < index('sequence') ? 'yoda' : 'sequence', index('yoda') < index('sequence') ? 'sequence' : 'yoda'];
    expect(offList(`${second},${first},${second}`)).toBe(`${first},${second}`);
    expect(offList('')).toBe('');
    expect(offList('yoda,bogus')).toBeUndefined();
    expect(withPass('', 'yoda', false)).toBe('yoda');
    expect(withPass(`${first},${second}`, first, true)).toBe(second);
  });

  it('sends --disable with the off list and round-trips it through the link', () => {
    expect(DEFAULTS.off).toBe('');
    expect(flagsFrom({ ...CLI_DEFAULTS, off: 'dead-code' })).toBe('--disable dead-code');
    expect(writeHash({ ...DEFAULTS, off: 'dead-code' })).toBe('#off=dead-code');
    expect(parseHash('#off=dead-code').settings.off).toBe('dead-code');
    const bad = parseHash('#off=bogus');
    expect(bad.settings.off).toBe('');
    expect(bad.problems).toHaveLength(1);
  });
});
