import { describe, expect, it } from 'vitest';
import { parseHash } from '../settings/hash';
import { LINK_CODE_CAP, LINK_INPUT_CAP, planLink, shareHash, sharedInputOf } from './link';

const pack = (text: string): string => `packed:${text}`;

describe('sharedInputOf', () => {
  it('reads the packed code and the file name beside the settings', () => {
    expect(sharedInputOf('#rename=unique&code=abc-_9&name=my%20file.js')).toEqual({ code: 'abc-_9', name: 'my file.js' });
  });

  it('gives null when the link holds no code', () => {
    expect(sharedInputOf('#rename=unique&unsafe=1')).toBeNull();
    expect(sharedInputOf('')).toBeNull();
  });

  it('leaves the name out when the link has none', () => {
    expect(sharedInputOf('#code=abc')).toEqual({ code: 'abc', name: null });
  });

  it('keeps a long name to its first 255 characters', () => {
    expect(sharedInputOf(`#code=abc&name=${'a'.repeat(400)}`)?.name).toBe('a'.repeat(255));
  });
});

describe('shareHash', () => {
  it('puts the code after the settings and names a file that is not untitled', () => {
    expect(shareHash('#rename=unique', 'abc', 'my file.js')).toBe('#rename=unique&code=abc&name=my%20file.js');
    expect(shareHash('', 'abc', 'untitled.js')).toBe('#code=abc');
  });

  it('is the settings hash alone without a code', () => {
    expect(shareHash('#rename=unique', null, 'x.js')).toBe('#rename=unique');
    expect(shareHash('', null, 'x.js')).toBe('');
  });

  it('round-trips through sharedInputOf and leaves parseHash without problems', () => {
    const hash = shareHash('#rename=unique', 'abc', 'a&b=c.js');
    expect(sharedInputOf(hash)).toEqual({ code: 'abc', name: 'a&b=c.js' });
    const parsed = parseHash(hash);
    expect(parsed.problems).toEqual([]);
    expect(parsed.settings.rename).toBe('unique');
  });
});

describe('planLink', () => {
  it('packs the input into the link', () => {
    const plan = planLink('#unsafe=0', { text: 'var a;', bytes: 6, name: 'untitled.js' }, pack);
    expect(plan).toEqual({ kind: 'code', hash: '#unsafe=0&code=packed:var a;', chars: 'packed:var a;'.length });
  });

  it('falls back to the settings for an empty input', () => {
    expect(planLink('#unsafe=0', { text: '', bytes: 0, name: 'untitled.js' }, pack)).toEqual({ kind: 'settings', hash: '#unsafe=0', reason: 'empty' });
  });

  it('falls back to the settings when the packed code is over the cap', () => {
    const text = 'x'.repeat(LINK_CODE_CAP);
    expect(planLink('', { text, bytes: text.length, name: 'untitled.js' }, pack)).toEqual({ kind: 'settings', hash: '', reason: 'large' });
  });

  it('does not pack an input over the byte cap', () => {
    let packed = false;
    const plan = planLink('', { text: 'x', bytes: LINK_INPUT_CAP + 1, name: 'untitled.js' }, () => {
      packed = true;
      return 'x';
    });
    expect(plan).toEqual({ kind: 'settings', hash: '', reason: 'large' });
    expect(packed).toBe(false);
  });
});
