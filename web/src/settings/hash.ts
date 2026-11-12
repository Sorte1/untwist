import { LINK_CODE_CAP, SHARED_KEYS } from '../share/link';
import { offList } from './passes';
import { DEFAULTS, RENAME_MODES, WASM_CHECKS, type Settings } from './settings';

export interface Parsed {
  readonly settings: Settings;
  readonly problems: readonly string[];
}

type Key = keyof Settings;
type Codec = { readonly read: (raw: string) => Settings[Key] | undefined; readonly write: (value: Settings[Key]) => string };

const MAX_COUNT = 1_000_000_000_000;
const MAX_ITERATIONS = 4_294_967_295;
const ECHO_CHARS = 80;
export const MAX_PROBLEMS = 20;
export const HASH_CAP = 2 * LINK_CODE_CAP + 4_096;

function count(raw: string): number | null | undefined {
  if (raw === '' || raw === 'none' || raw === 'unlimited') {
    return null;
  }
  if (!/^[1-9][0-9]*$/.test(raw)) {
    return undefined;
  }
  const value = Number(raw);
  return value <= MAX_COUNT ? value : undefined;
}

const flag: Codec = {
  read: (raw) => (raw === '1' ? true : raw === '0' ? false : undefined),
  write: (value) => (value === true ? '1' : '0'),
};

const counted: Codec = {
  read: count,
  write: (value) => (value === null ? 'none' : String(value)),
};

function choice(values: readonly string[]): Codec {
  return { read: (raw) => (values.includes(raw) ? (raw as Settings[Key]) : undefined), write: (value) => String(value) };
}

const CODECS: Readonly<Record<Key, Codec>> = {
  unpack: flag,
  jsx: flag,
  unsafe: flag,
  rename: choice(RENAME_MODES),
  fuel: { read: count, write: (value) => (value === null ? 'unlimited' : String(value)) },
  deob: flag,
  unmin: flag,
  human: flag,
  iter: {
    read: (raw) => {
      const value = count(raw);
      return typeof value === 'number' && value > MAX_ITERATIONS ? undefined : value;
    },
    write: (value) => (value === null ? 'none' : String(value)),
  },
  single: flag,
  off: { read: offList, write: (value) => String(value) },
  wasm: choice(WASM_CHECKS),
  view: choice(['output', 'diff', 'steps']),
  wrap: flag,
  theme: { read: (raw) => (raw === 'dark' || raw === 'light' ? raw : undefined), write: (value) => String(value) },
  input: choice(['shown', 'hidden']),
};

const KEYS = Object.keys(CODECS) as Key[];

function decoded(raw: string): string {
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

export function writeHash(settings: Settings): string {
  const parts = KEYS.filter((key) => settings[key] !== DEFAULTS[key]).map(
    (key) => `${key}=${encodeURIComponent(CODECS[key].write(settings[key]))}`,
  );
  return parts.length === 0 ? '' : `#${parts.join('&')}`;
}

function shown(text: string): string {
  return text.length > ECHO_CHARS ? `${text.slice(0, ECHO_CHARS)}…` : text;
}

function readPart(rawKey: string, rawValue: string): string | Readonly<[Key, Settings[Key]]> | null {
  const key = decoded(rawKey);
  if (SHARED_KEYS.includes(key)) {
    return null;
  }
  if (!Object.hasOwn(CODECS, key)) {
    return `Unknown setting "${shown(key)}" in the link was ignored.`;
  }
  const known = key as Key;
  const value = CODECS[known].read(decoded(rawValue));
  if (value === undefined) {
    return `Setting "${known}" has an invalid value "${shown(decoded(rawValue))}"; the default is used.`;
  }
  return [known, value];
}

export function parseHash(hash: string): Parsed {
  const whole = hash.startsWith('#') ? hash.slice(1) : hash;
  const body = whole.slice(0, HASH_CAP);
  const problems: string[] = [];
  let unlisted = 0;
  const entries: Array<Readonly<[Key, Settings[Key]]>> = [];
  for (const part of body.split('&')) {
    if (part === '') {
      continue;
    }
    const split = part.indexOf('=');
    const found = split === -1 ? readPart(part, '') : readPart(part.slice(0, split), part.slice(split + 1));
    if (typeof found === 'string') {
      if (problems.length < MAX_PROBLEMS) {
        problems.push(found);
      } else {
        unlisted += 1;
      }
    } else if (found !== null) {
      entries.push(found);
    }
  }
  if (unlisted > 0) {
    problems.push(`And ${unlisted.toLocaleString('en-US')} more problems with the settings in the link.`);
  }
  if (whole.length > HASH_CAP) {
    problems.push(`The link is longer than ${HASH_CAP.toLocaleString('en-US')} characters; the settings after that were ignored.`);
  }
  return { settings: { ...DEFAULTS, ...Object.fromEntries(entries) } as Settings, problems };
}

export function settingsFromHash(hash: string, parse: (hash: string) => Parsed = parseHash): Parsed {
  try {
    return parse(hash);
  } catch {
    return { settings: DEFAULTS, problems: ['The settings in the link could not be read; the defaults are used.'] };
  }
}
