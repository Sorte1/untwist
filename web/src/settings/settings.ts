export type RenameMode = 'none' | 'simple' | 'smart' | 'unique' | 'mangle';
export type View = 'output' | 'diff' | 'steps';
export type ThemeChoice = 'dark' | 'light';
export type InputVisibility = 'shown' | 'hidden';
export type WasmCheck = 'off' | 'on' | 'auto';

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
  readonly wasm: WasmCheck;
  readonly view: View;
  readonly wrap: boolean;
  readonly theme: ThemeChoice | null;
  readonly input: InputVisibility;
  readonly libs: boolean;
}

export const RENAME_MODES: readonly RenameMode[] = ['none', 'simple', 'smart', 'unique', 'mangle'];
export const WASM_CHECKS: readonly WasmCheck[] = ['off', 'on', 'auto'];
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
  wasm: 'off',
  view: 'output',
  wrap: false,
  theme: null,
  input: 'shown',
  libs: false,
};

export const CLI_DEFAULTS: Settings = { ...DEFAULTS, unsafe: false, libs: true };

export const LIBRARY_PASSES: readonly string[] = ['library-names', 'library-rename'];

export const RENAME_HINTS: Readonly<Record<RenameMode, string>> = {
  none: 'Keep identifiers exactly as in the input (the default).',
  simple: 'Per-function a, b, … sequences.',
  smart: 'Names from evidence: the original names of matched library code such as core-js (with Library names on), i for loop counters, item and index in callbacks, err in catch, canvas for createElement("canvas"); human-written names stay.',
  unique: 'Program-wide readable names such as fn1, arr2, handler3.',
  mangle: 'Short mangled names (a, b, … aa, ab) chosen program-wide.',
};

export const WASM_CHECK_HINTS: Readonly<Record<WasmCheck, string>> = {
  off: 'Only say when the output may hold embedded WebAssembly (the default).',
  on: 'Reparse every output and extract any embedded WebAssembly module.',
  auto: 'Extract byte arrays and base64 text that hold a WebAssembly module with imports or exports.',
};

export function sameSettings(a: Settings, b: Settings): boolean {
  return (Object.keys(DEFAULTS) as Array<keyof Settings>).every((key) => a[key] === b[key]);
}

const PANEL_KEYS: ReadonlyArray<keyof Settings> = ['unpack', 'jsx', 'libs', 'rename', 'fuel', 'deob', 'unmin', 'human', 'iter', 'single', 'off', 'wasm'];

export function changedCount(settings: Settings): number {
  return PANEL_KEYS.filter((key) => settings[key] !== DEFAULTS[key]).length;
}
