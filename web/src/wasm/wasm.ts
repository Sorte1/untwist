import { hostRefusal } from './address';

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

export async function limitedBody(response: Response, limit: number): Promise<Uint8Array<ArrayBuffer> | null> {
  const declared = Number(response.headers.get('content-length') ?? '');
  if (declared > limit) {
    return null;
  }
  const reader = response.body?.getReader();
  if (reader === undefined) {
    return new Uint8Array(0);
  }
  const chunks: Uint8Array[] = [];
  let total = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    total += value.byteLength;
    if (total > limit) {
      await reader.cancel();
      return null;
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(total);
  chunks.reduce((offset, chunk) => {
    bytes.set(chunk, offset);
    return offset + chunk.byteLength;
  }, 0);
  return bytes;
}

export function withSupplied(current: readonly SuppliedWasm[], added: readonly SuppliedWasm[]): SuppliedWasm[] {
  const names = new Set(added.map((file) => file.name));
  return [...current.filter((file) => !names.has(file.name)), ...added];
}

export type FetchTarget =
  | { readonly ok: true; readonly href: string; readonly host: string }
  | { readonly ok: false; readonly reason: string };

const UNREQUESTABLE = /[^\x21-\x7e]|\\/;
const WITH_AUTHORITY = /^[a-z][a-z0-9+.-]*:\/\/[^/?#]/i;

export function hasScheme(text: string): boolean {
  return /^[a-z][a-z0-9+.-]*:/i.test(text);
}

function refused(reason: string): FetchTarget {
  return { ok: false, reason };
}

function parsed(text: string, base?: string): URL | null {
  try {
    return new URL(text, base);
  } catch {
    return null;
  }
}

export function checkedUrl(text: string, base?: string): FetchTarget {
  const shown = base === undefined ? text : `${text} against ${base}`;
  const malformed = UNREQUESTABLE.test(text) || (hasScheme(text) && !WITH_AUTHORITY.test(text));
  const url = malformed ? null : parsed(text, base);
  if (url === null || (url.protocol !== 'http:' && url.protocol !== 'https:') || url.hostname === '') {
    return refused(`${shown} is not an http(s) URL that can be requested`);
  }
  if (url.username !== '' || url.password !== '') {
    return refused(`${shown} carries a user name or password`);
  }
  const host = hostRefusal(url.hostname);
  if (host !== null) {
    return refused(`${host}; the playground does not fetch from local or private addresses, download the file and drop it here`);
  }
  url.hash = '';
  return { ok: true, href: url.href, host: url.host };
}

export function fetchTarget(reference: string, base: string | null): FetchTarget {
  if (hasScheme(reference)) {
    return checkedUrl(reference);
  }
  if (base === null) {
    return refused(`${reference} is relative; enter the page or script URL it is loaded from`);
  }
  const trimmed = base.trim();
  const checkedBase = checkedUrl(trimmed);
  if (!checkedBase.ok) {
    return checkedBase;
  }
  return checkedUrl(reference, checkedBase.href);
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
