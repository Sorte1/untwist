import type { SuppliedWasm } from '../wasm/wasm';

export type TrapKind = 'stack' | 'other';

export interface RunFields {
  readonly stdout: string;
  readonly stderr: string;
  readonly code: number;
  readonly limits: string;
}

export interface RunResult extends RunFields {
  readonly report: string;
  readonly bundleJson: string | null;
  readonly modules: readonly string[];
  readonly trap: TrapKind | null;
  readonly engineMs: number;
  readonly memoryBytes: number;
  readonly workerMs: number;
  readonly snapshots?: readonly (string | null)[];
  readonly decodedStrings?: string;
  readonly decodedCount?: number;
  readonly links?: string;
  readonly wasm?: string;
  readonly wasmBytes?: readonly Uint8Array<ArrayBuffer>[];
  readonly tablesLoaded?: string;
  readonly libraryTables?: LibraryTablesUsed;
}

export interface LibraryTablesUsed {
  readonly fetched: readonly string[];
  readonly failed: readonly string[];
  readonly reran: boolean;
  readonly scanMs: number;
  readonly loadMs: number;
}

export type WorkerRequest =
  | { readonly kind: 'init'; readonly module: WebAssembly.Module }
  | { readonly kind: 'run'; readonly source: string; readonly flags: string; readonly refused: boolean; readonly extras: string; readonly wasm: readonly SuppliedWasm[]; readonly tables?: readonly Uint8Array<ArrayBuffer>[] };

export type WorkerReply =
  | { readonly kind: 'ready' }
  | { readonly kind: 'stage'; readonly name: string }
  | { readonly kind: 'failed'; readonly message: string }
  | { readonly kind: 'trap'; readonly trap: TrapKind; readonly message: string }
  | { readonly kind: 'done'; readonly result: Omit<RunResult, 'trap' | 'workerMs' | 'libraryTables'> };
