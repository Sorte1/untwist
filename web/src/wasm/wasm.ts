export const MAX_WASM_BYTES = 32 * 1024 * 1024;
export const ALWAYS_FETCH_KEY = 'untwist.alwaysFetchWasm';

export type WasmOrigin = 'embedded' | 'supplied' | 'fetched';

export interface WasmFileInfo {
  readonly path: string;
  readonly origin: WasmOrigin;
  readonly url?: string;
  readonly byteLength: number;
  readonly imports: number;
  readonly exports: number;
  readonly renamed: number;
}

export interface WasmReferenceInfo {
  readonly name: string;
  readonly url: string;
}

export interface WasmReport {
  readonly files: readonly WasmFileInfo[];
  readonly unresolved: readonly WasmReferenceInfo[];
  readonly unchecked: boolean;
}

export interface SuppliedWasm {
  readonly name: string;
  readonly bytes: Uint8Array<ArrayBuffer>;
  readonly url: string | null;
}

export const NO_WASM: WasmReport = { files: [], unresolved: [], unchecked: false };

const WASM_MAGIC = [0x00, 0x61, 0x73, 0x6d];

export function parseWasmReport(json: string): WasmReport {
  if (json === '') {
    return NO_WASM;
  }
  try {
    const parsed = JSON.parse(json) as Partial<WasmReport>;
    return { files: parsed.files ?? [], unresolved: parsed.unresolved ?? [], unchecked: parsed.unchecked === true };
  } catch {
    return NO_WASM;
  }
}

export const UNCHECKED_WARNING = 'This file may contain embedded WebAssembly. Set Wasm check to Auto or On to extract it.';

export function isWasmName(name: string): boolean {
  return name.toLowerCase().endsWith('.wasm');
}

export function hasWasmHeader(bytes: Uint8Array): boolean {
  return bytes.length >= 8 && WASM_MAGIC.every((byte, index) => bytes[index] === byte) && bytes[4] === 1 && bytes[5] === 0 && bytes[6] === 0 && bytes[7] === 0;
}

export function checkedWasm(name: string, bytes: Uint8Array): string | null {
  if (bytes.length > MAX_WASM_BYTES) {
    return `${name} is over the ${MAX_WASM_BYTES / (1024 * 1024)} MiB wasm size limit`;
  }
  return hasWasmHeader(bytes) ? null : `${name} is not a wasm module (no \\0asm version 1 header)`;
}

export function withSupplied(current: readonly SuppliedWasm[], added: readonly SuppliedWasm[]): SuppliedWasm[] {
  const names = new Set(added.map((file) => file.name));
  return [...current.filter((file) => !names.has(file.name)), ...added];
}

export function isAbsoluteUrl(url: string): boolean {
  return /^https?:\/\//i.test(url);
}

export function resolvedUrl(url: string, base: string | null): string | null {
  if (isAbsoluteUrl(url)) {
    return url;
  }
  if (base === null || !isAbsoluteUrl(base.trim())) {
    return null;
  }
  try {
    return new URL(url, base.trim()).href;
  } catch {
    return null;
  }
}

function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function alwaysFetchWasm(): boolean {
  try {
    return storage()?.getItem(ALWAYS_FETCH_KEY) === '1';
  } catch {
    return false;
  }
}

export function setAlwaysFetchWasm(on: boolean): void {
  try {
    if (on) {
      storage()?.setItem(ALWAYS_FETCH_KEY, '1');
    } else {
      storage()?.removeItem(ALWAYS_FETCH_KEY);
    }
  } catch {
    return;
  }
}
