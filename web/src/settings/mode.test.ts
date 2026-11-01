import { describe, expect, it } from 'vitest';
import { CLI_DEFAULTS, DEFAULTS } from './settings';
import { JSX_HINTS, jsxInactive, modeFromFlags, modeLabel, modeOf, steppedMode, UNSAFE_WARNING } from './mode';

describe('the Safe / Unsafe mode', () => {
  it('reads the mode from the unsafe setting; the site opens in Unsafe, the CLI default stays Safe', () => {
    expect(modeOf(DEFAULTS)).toBe('unsafe');
    expect(modeOf(CLI_DEFAULTS)).toBe('safe');
    expect(modeOf({ ...DEFAULTS, unsafe: false })).toBe('safe');
  });

  it('reads the mode that produced an output from its CLI flags', () => {
    expect(modeFromFlags(null)).toBeNull();
    expect(modeFromFlags('')).toBe('safe');
    expect(modeFromFlags('--no-unpack --rename unique')).toBe('safe');
    expect(modeFromFlags('--no-jsx --unsafe --single-file')).toBe('unsafe');
    expect(modeFromFlags('--unsafe-ish')).toBe('safe');
  });

  it('labels a mode, or a dash when nothing has run', () => {
    expect(modeLabel('safe')).toBe('Safe');
    expect(modeLabel('unsafe')).toBe('Unsafe');
    expect(modeLabel(null)).toBe('—');
  });

  it('moves between the two radios with the arrow keys, Home and End, and ignores other keys', () => {
    expect(steppedMode('safe', 'ArrowRight')).toBe('unsafe');
    expect(steppedMode('safe', 'ArrowDown')).toBe('unsafe');
    expect(steppedMode('unsafe', 'ArrowRight')).toBe('safe');
    expect(steppedMode('unsafe', 'ArrowLeft')).toBe('safe');
    expect(steppedMode('safe', 'ArrowUp')).toBe('unsafe');
    expect(steppedMode('unsafe', 'Home')).toBe('safe');
    expect(steppedMode('safe', 'End')).toBe('unsafe');
    expect(steppedMode('safe', 'Tab')).toBeNull();
    expect(steppedMode('safe', 'a')).toBeNull();
  });

  it('marks JSX inactive when it is on but the mode is Safe', () => {
    expect(jsxInactive(CLI_DEFAULTS)).toBe(true);
    expect(jsxInactive(DEFAULTS)).toBe(false);
    expect(jsxInactive({ ...CLI_DEFAULTS, jsx: false })).toBe(false);
    expect(JSX_HINTS.inactive).toMatch(/inactive/i);
    expect(JSX_HINTS.inactive).toMatch(/Unsafe/);
    expect(JSX_HINTS.active).not.toMatch(/inactive/i);
  });

  it('warns about what Unsafe assumes', () => {
    expect(UNSAFE_WARNING).toBe('may change behaviour in edge cases; assumes built-ins are not modified');
  });
});
