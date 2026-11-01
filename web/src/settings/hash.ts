import { offList } from './passes';
import { DEFAULTS, RENAME_MODES, type Settings } from './settings';

export interface Parsed {
  readonly settings: Settings;
  readonly problems: readonly string[];
}

type Key = keyof Settings;
type Codec = { readonly read: (raw: string) => Settings[Key] | undefined; readonly write: (value: Settings[Key]) => string };

const MAX_COUNT = 1_000_000_000_000;
const MAX_ITERATIONS = 4_294_967_295;

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
  view: choice(['output', 'diff']),
  theme: { read: (raw) => (raw === 'dark' || raw === 'light' ? raw : undefined), write: (value) => String(value) },
  input: choice(['shown', 'hidden']),
};

const KEYS = Object.keys(CODECS) as Key[];

export function writeHash(settings: Settings): string {
  const parts = KEYS.filter((key) => settings[key] !== DEFAULTS[key]).map(
    (key) => `${key}=${encodeURIComponent(CODECS[key].write(settings[key]))}`,
  );
  return parts.length === 0 ? '' : `#${parts.join('&')}`;
}

export function parseHash(hash: string): Parsed {
  const body = hash.startsWith('#') ? hash.slice(1) : hash;
  const problems: string[] = [];
  const entries: Array<[Key, Settings[Key]]> = [];
  for (const part of body.split('&').filter((item) => item !== '')) {
    const [rawKey = '', rawValue = ''] = part.split('=', 2);
    const key = decodeURIComponent(rawKey) as Key;
    if (!(key in CODECS)) {
      problems.push(`Unknown setting "${decodeURIComponent(rawKey)}" in the link was ignored.`);
      continue;
    }
    const value = CODECS[key].read(decodeURIComponent(rawValue));
    if (value === undefined) {
      problems.push(`Setting "${key}" has an invalid value "${decodeURIComponent(rawValue)}"; the default is used.`);
      continue;
    }
    entries.push([key, value]);
  }
  return { settings: { ...DEFAULTS, ...Object.fromEntries(entries) } as Settings, problems };
}
