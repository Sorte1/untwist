import { describe, expect, it } from 'vitest';
import { flagsFrom } from './flags';
import { HASH_CAP, MAX_PROBLEMS, parseHash, settingsFromHash, writeHash } from './hash';
import { changedCount, CLI_DEFAULTS, DEFAULT_FUEL, DEFAULTS, RENAME_MODES, WASM_CHECKS, type Settings } from './settings';
import cases from './flags.json';

function pick<T>(values: readonly T[], seed: number): T {
  return values[seed % values.length] as T;
}

function generated(seed: number): Settings {
  const bit = (shift: number) => ((seed >> shift) & 1) === 1;
  return {
    unpack: bit(0),
    jsx: bit(1),
    unsafe: bit(2),
    rename: pick(RENAME_MODES, seed),
    fuel: pick([null, 1, 1000, 50_000_000], seed >> 2),
    deob: bit(4),
    unmin: bit(5),
    human: bit(6),
    iter: pick([null, 1, 7, 4_294_967_295], seed >> 3),
    single: bit(7),
    off: pick(['', 'dead-code', 'layers'], seed >> 5),
    wasm: pick(WASM_CHECKS, seed >> 6),
    view: bit(8) ? 'diff' : 'output',
    wrap: bit(10),
    theme: pick([null, 'dark', 'light'] as const, seed >> 4),
    input: bit(9) ? 'hidden' : 'shown',
  };
}

describe('flagsFrom', () => {
  it('emits nothing for the CLI defaults, and --unsafe for the site defaults, which open in Unsafe', () => {
    expect(flagsFrom(CLI_DEFAULTS)).toBe('');
    expect(flagsFrom(DEFAULTS)).toBe('--unsafe');
    expect(CLI_DEFAULTS).toEqual({ ...DEFAULTS, unsafe: false });
  });

  it('emits the flag words of every case in the fixture shared with the cargo test', () => {
    for (const item of cases) {
      expect(flagsFrom({ ...CLI_DEFAULTS, ...(item.settings as Partial<Settings>) })).toBe(item.flags);
    }
  });

  it('sends an unlimited fuel for "No limit" and nothing for the default fuel', () => {
    expect(DEFAULTS.fuel).toBe(DEFAULT_FUEL);
    expect(flagsFrom({ ...CLI_DEFAULTS, fuel: null })).toBe('--fuel unlimited');
    expect(flagsFrom({ ...CLI_DEFAULTS, fuel: DEFAULT_FUEL })).toBe('');
    expect(flagsFrom({ ...CLI_DEFAULTS, fuel: 1000 })).toBe('--fuel 1000');
  });

  it('never emits -j', () => {
    for (let seed = 0; seed < 1024; seed += 1) {
      expect(flagsFrom(generated(seed)).split(' ')).not.toContain('-j');
    }
  });
});

describe('the settings hash', () => {
  it('round-trips generated settings', () => {
    for (let seed = 0; seed < 1024; seed += 1) {
      const settings = generated(seed);
      const parsed = parseHash(writeHash(settings));
      expect(parsed.problems).toEqual([]);
      expect(parsed.settings).toEqual(settings);
    }
  });

  it('writes only what differs from the defaults', () => {
    expect(writeHash(DEFAULTS)).toBe('');
    expect(writeHash({ ...DEFAULTS, rename: 'unique' })).toBe('#rename=unique');
    expect(writeHash({ ...DEFAULTS, unsafe: false })).toBe('#unsafe=0');
  });

  it('keeps "No limit" in the link as fuel=unlimited and still reads fuel=none', () => {
    expect(writeHash({ ...DEFAULTS, fuel: null })).toBe('#fuel=unlimited');
    expect(parseHash('#fuel=unlimited').settings.fuel).toBeNull();
    expect(parseHash('#fuel=none').settings.fuel).toBeNull();
    expect(parseHash('').settings.fuel).toBe(DEFAULT_FUEL);
  });

  it('opens in Unsafe without a mode in the link, and an explicit mode in the link wins', () => {
    expect(parseHash('').settings.unsafe).toBe(true);
    expect(parseHash('#rename=simple').settings.unsafe).toBe(true);
    expect(parseHash('#unsafe=0').settings.unsafe).toBe(false);
    expect(parseHash('#unsafe=1').settings.unsafe).toBe(true);
  });

  it('falls back to the default on an unknown key or an invalid value, with one problem each', () => {
    const parsed = parseHash('#bogus=1&unsafe=yes&fuel=-3&rename=full&iter=0&jsx=0');
    expect(parsed.settings).toEqual({ ...DEFAULTS, jsx: false });
    expect(parsed.problems).toHaveLength(5);
  });
});

describe('changedCount', () => {
  it('counts the transform settings that differ from the defaults, not the mode, view, wrap, theme or input', () => {
    expect(changedCount(DEFAULTS)).toBe(0);
    expect(changedCount({ ...DEFAULTS, unsafe: false, view: 'diff', wrap: true, theme: 'light', input: 'hidden' })).toBe(0);
    expect(changedCount({ ...DEFAULTS, unpack: false, rename: 'unique', fuel: null })).toBe(3);
    expect(changedCount({ ...DEFAULTS, off: 'dead-code,layers', iter: 3, single: true })).toBe(3);
  });
});

describe('a malformed link', () => {
  it('reports a broken percent escape as a problem instead of throwing', () => {
    const parsed = parseHash('#unpack=%&%zz=1&rename=unique');
    expect(parsed.settings.rename).toBe('unique');
    expect(parsed.settings.unpack).toBe(DEFAULTS.unpack);
    expect(parsed.problems).toEqual([
      'Setting "unpack" has an invalid value "%"; the default is used.',
      'Unknown setting "%zz" in the link was ignored.',
    ]);
  });

  it('treats the names Object.prototype carries as unknown settings', () => {
    for (const name of ['toString', 'constructor', '__proto__', 'valueOf', 'hasOwnProperty']) {
      const parsed = parseHash(`#${name}=1&rename=unique`);
      expect(parsed.settings).toEqual({ ...DEFAULTS, rename: 'unique' });
      expect(parsed.problems).toEqual([`Unknown setting "${name}" in the link was ignored.`]);
    }
  });

  it('lists a few problems and counts the rest when the link holds many unknown settings', () => {
    const parsed = parseHash(`#${'a&'.repeat(200_000)}unsafe=0`);
    expect(parsed.problems.length).toBeLessThanOrEqual(MAX_PROBLEMS + 2);
    expect(parsed.problems.slice(0, MAX_PROBLEMS)).toEqual(Array.from({ length: MAX_PROBLEMS }, () => 'Unknown setting "a" in the link was ignored.'));
    expect(parsed.problems.at(-1)).toMatch(/^The link is longer than [\d,]+ characters; the settings after that were ignored\.$/);
    expect(parsed.problems.at(-2)).toMatch(/^And [\d,]+ more problems with the settings in the link\.$/);
  });

  it('reads the settings of a link that stays under the length cap, after a long code part', () => {
    const parsed = parseHash(`#code=${'x'.repeat(HASH_CAP - 40)}&unsafe=0&rename=unique`);
    expect(parsed.problems).toEqual([]);
    expect(parsed.settings).toEqual({ ...DEFAULTS, unsafe: false, rename: 'unique' });
  });

  it('cuts a long key or value in a problem', () => {
    const key = 'k'.repeat(10_000);
    const parsed = parseHash(`#${key}=1&rename=${'v'.repeat(10_000)}`);
    expect(parsed.problems).toHaveLength(2);
    for (const problem of parsed.problems) {
      expect(problem.length).toBeLessThan(200);
    }
    expect(parsed.problems[0]).toBe(`Unknown setting "${'k'.repeat(80)}…" in the link was ignored.`);
  });
});

describe('settingsFromHash', () => {
  it('gives the parsed settings of a well-formed link', () => {
    expect(settingsFromHash('#unsafe=0')).toEqual(parseHash('#unsafe=0'));
  });

  it('starts with the defaults and one problem when reading the link throws', () => {
    const parsed = settingsFromHash('#unsafe=0', () => {
      throw new Error('boom');
    });
    expect(parsed.settings).toEqual(DEFAULTS);
    expect(parsed.problems).toEqual(['The settings in the link could not be read; the defaults are used.']);
  });
});
