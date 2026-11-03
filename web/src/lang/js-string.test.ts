import { describe, expect, it } from 'vitest';
import { decodeJsString, hasEscapes } from './js-string';

describe('decoding a string literal', () => {
  it('reads hex, unicode and code point escapes', () => {
    expect(decodeJsString('"\\x68\\x69"')).toBe('hi');
    expect(decodeJsString("'\\u0041\\u{1F600}'")).toBe('A😀');
  });

  it('reads the single-character escapes and legacy octal', () => {
    expect(decodeJsString('"a\\nb\\tc\\\\d\\"e"')).toBe('a\nb\tc\\d"e');
    expect(decodeJsString('"\\101\\0"')).toBe('A\0');
    expect(decodeJsString('"\\q"')).toBe('q');
  });

  it('drops a line continuation', () => {
    expect(decodeJsString('"a\\\nb"')).toBe('ab');
  });

  it('refuses a broken escape or a text that is not a literal', () => {
    expect(decodeJsString('"\\x4"')).toBeNull();
    expect(decodeJsString('"\\u{110000}"')).toBeNull();
    expect(decodeJsString('`tpl`')).toBeNull();
    expect(decodeJsString('"open')).toBeNull();
  });

  it('knows when a literal has escapes', () => {
    expect(hasEscapes('"\\x41"')).toBe(true);
    expect(hasEscapes('"plain"')).toBe(false);
  });
});
