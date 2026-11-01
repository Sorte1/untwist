export type RenameMode = 'none' | 'simple' | 'unique' | 'mangle';
export type View = 'output' | 'diff';
export type ThemeChoice = 'dark' | 'light';
export type InputVisibility = 'shown' | 'hidden';

export interface Settings {
  readonly unpack: boolean;
  readonly jsx: boolean;
  readonly unsafe: boolean;
  readonly rename: RenameMode;
  readonly fuel: number | null;
  readonly deob: boolean;
  readonly unmin: boolean;
  readonly human: boolean;
  readonly iter: number | null;
  readonly single: boolean;
  readonly off: string;
  readonly view: View;
  readonly theme: ThemeChoice | null;
  readonly input: InputVisibility;
}

export const RENAME_MODES: readonly RenameMode[] = ['none', 'simple', 'unique', 'mangle'];
export const DEFAULT_FUEL = 50_000_000;

export const DEFAULTS: Settings = {
  unpack: true,
  jsx: true,
  unsafe: true,
  rename: 'none',
  fuel: DEFAULT_FUEL,
  deob: true,
  unmin: true,
  human: false,
  iter: null,
  single: false,
  off: '',
  view: 'output',
  theme: null,
  input: 'shown',
};

export const CLI_DEFAULTS: Settings = { ...DEFAULTS, unsafe: false };

export const RENAME_HINTS: Readonly<Record<RenameMode, string>> = {
  none: 'Keep identifiers exactly as in the input (the default).',
  simple: 'Per-function a, b, … sequences.',
  unique: 'Program-wide readable names such as fn1, arr2, handler3.',
  mangle: 'Short mangled names (a, b, … aa, ab) chosen program-wide.',
};

export function sameSettings(a: Settings, b: Settings): boolean {
  return (Object.keys(DEFAULTS) as Array<keyof Settings>).every((key) => a[key] === b[key]);
}
