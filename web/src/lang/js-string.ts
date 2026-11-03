const SIMPLE: Readonly<Record<string, string>> = { n: '\n', t: '\t', r: '\r', b: '\b', f: '\f', v: '\v' };
const LINE_BREAKS = new Set(['\n', '\r', ' ', ' ']);
const HEX = /^[0-9a-fA-F]+$/;

function hexUnit(text: string): string | null {
  if (!HEX.test(text)) {
    return null;
  }
  const code = Number.parseInt(text, 16);
  return code > 0x10ffff ? null : String.fromCodePoint(code);
}

function unicode(raw: string, at: number): readonly [string, number] | null {
  if (raw[at] === '{') {
    const close = raw.indexOf('}', at);
    const unit = close < 0 ? null : hexUnit(raw.slice(at + 1, close));
    return unit === null ? null : [unit, close + 1];
  }
  const unit = hexUnit(raw.slice(at, at + 4));
  return unit === null || raw.slice(at, at + 4).length < 4 ? null : [unit, at + 4];
}

function octal(raw: string, at: number): readonly [string, number] {
  const match = /^[0-3]?[0-7]{1,2}|^[0-7]/.exec(raw.slice(at, at + 3));
  const digits = match?.[0] ?? '0';
  return [String.fromCharCode(Number.parseInt(digits, 8)), at + digits.length];
}

function escaped(raw: string, at: number): readonly [string, number] | null {
  const char = raw[at];
  if (char === undefined) {
    return null;
  }
  if (char in SIMPLE) {
    return [SIMPLE[char] ?? '', at + 1];
  }
  if (char === 'x') {
    const unit = raw.slice(at + 1, at + 3).length === 2 ? hexUnit(raw.slice(at + 1, at + 3)) : null;
    return unit === null ? null : [unit, at + 3];
  }
  if (char === 'u') {
    return unicode(raw, at + 1);
  }
  if (/[0-7]/.test(char)) {
    return octal(raw, at);
  }
  if (char === '\r' && raw[at + 1] === '\n') {
    return ['', at + 2];
  }
  return [LINE_BREAKS.has(char) ? '' : char, at + 1];
}

export function decodeJsString(literal: string): string | null {
  const quote = literal[0];
  if ((quote !== '"' && quote !== "'") || literal.length < 2 || literal[literal.length - 1] !== quote) {
    return null;
  }
  const raw = literal.slice(1, -1);
  let value = '';
  let at = 0;
  while (at < raw.length) {
    const slash = raw.indexOf('\\', at);
    if (slash < 0) {
      value += raw.slice(at);
      break;
    }
    value += raw.slice(at, slash);
    const next = escaped(raw, slash + 1);
    if (next === null) {
      return null;
    }
    value += next[0];
    at = next[1];
  }
  return value;
}

export function hasEscapes(literal: string): boolean {
  return literal.includes('\\');
}
