import { initSync, library_tables, library_wanted, load_library_collection, load_library_shard, run_report_with_wasm, WasmFiles } from './pkg/untwist_wasm.js';
import type { CollectionFiles, FetchedShard } from '../libraries/collection';
import type { CollectionShard } from '../libraries/collection-format';
import { LibraryStage, type LibraryApi } from './library-stage';
import type { SuppliedWasm } from '../wasm/wasm';
import { MARK } from '../progress';
import type { TrapKind, WorkerReply, WorkerRequest } from './types';

const LIBRARY_API: LibraryApi = { attach: load_library_collection, wanted: library_wanted, load: load_library_shard };

let memory: WebAssembly.Memory | null = null;
let requests = 0;
const pending = new Map<number, (results: readonly (FetchedShard | null)[]) => void>();

function reply(message: WorkerReply): void {
  self.postMessage(message);
}

function stage(name: string): void {
  reply({ kind: 'stage', name, at: performance.timeOrigin + performance.now() });
}

(globalThis as { untwistStage?: (name: string) => void }).untwistStage = stage;

function fetchShards(shards: readonly CollectionShard[]): Promise<readonly (FetchedShard | null)[]> {
  requests += 1;
  const request = requests;
  return new Promise((resolve) => {
    pending.set(request, resolve);
    reply({ kind: 'fetch-shards', request, shards });
  });
}

function trapKind(error: unknown): TrapKind | null {
  if (error instanceof RangeError && /Maximum call stack size exceeded/.test(error.message)) {
    return 'stack';
  }
  if (error instanceof WebAssembly.RuntimeError) {
    return /memory access out of bounds/.test(error.message) ? 'stack' : 'other';
  }
  return null;
}

function init(module: WebAssembly.Module): void {
  try {
    memory = initSync({ module }).memory;
    reply({ kind: 'ready' });
  } catch (error) {
    reply({ kind: 'failed', message: String(error) });
  }
}

function modulesOf(output: { module_count(): number; module_code(index: number): string | undefined }): string[] {
  const modules: string[] = [];
  const count = output.module_count();
  for (let index = 0; index < count; index += 1) {
    modules.push(output.module_code(index) ?? '');
  }
  return modules;
}

function snapshotsOf(output: { snapshot_count(): number; snapshot_code(index: number): string | undefined }): Array<string | null> {
  const codes: Array<string | null> = [];
  const count = output.snapshot_count();
  for (let index = 0; index < count; index += 1) {
    codes.push(output.snapshot_code(index) ?? null);
  }
  return codes;
}

function wasmBytesOf(output: { wasm_bytes(index: number): Uint8Array | undefined }, count: number): Uint8Array<ArrayBuffer>[] {
  const bytes: Uint8Array<ArrayBuffer>[] = [];
  for (let index = 0; index < count; index += 1) {
    bytes.push(new Uint8Array(output.wasm_bytes(index) ?? []));
  }
  return bytes;
}

function wasmCount(json: string): number {
  try {
    return (JSON.parse(json) as { files?: unknown[] }).files?.length ?? 0;
  } catch {
    return 0;
  }
}

type Output = ReturnType<typeof run_report_with_wasm>;

function engineRun(source: string, flags: string, refused: boolean, extras: string, wasm: readonly SuppliedWasm[]): Output {
  const files = new WasmFiles();
  for (const file of wasm) {
    files.add(file.name, file.bytes, file.url ?? undefined);
  }
  const output = run_report_with_wasm(source, flags, refused, extras, files);
  files.free();
  return output;
}

async function withLibraries(libraryStage: LibraryStage, source: string, once: () => Output): Promise<{ readonly output: Output; readonly reran: boolean }> {
  await libraryStage.take([source], MARK.scan);
  const output = once();
  const found = await libraryStage.take([output.stdout, ...modulesOf(output)], MARK.scanOutput);
  if (found === 0) {
    return { output, reran: false };
  }
  output.free();
  stage(MARK.rerun);
  return { output: once(), reran: true };
}

async function run(source: string, flags: string, refused: boolean, extras: string, wasm: readonly SuppliedWasm[], libraries: CollectionFiles | null): Promise<void> {
  const started = performance.now();
  try {
    const once = () => engineRun(source, flags, refused, extras, wasm);
    const libraryStage = libraries === null ? null : new LibraryStage(LIBRARY_API, libraries, stage, fetchShards);
    const { output, reran } = libraryStage === null ? { output: once(), reran: false } : await withLibraries(libraryStage, source, once);
    stage(MARK.transfer);
    const wasmJson = output.wasm_json();
    const result = {
      stdout: output.stdout,
      stderr: output.stderr,
      code: output.code,
      limits: output.limits,
      report: output.report_json(),
      bundleJson: output.bundle_json() ?? null,
      modules: modulesOf(output),
      snapshots: snapshotsOf(output),
      decodedStrings: output.decoded_strings_json(),
      decodedCount: output.decoded_count(),
      links: output.links_json(),
      wasm: wasmJson,
      wasmBytes: wasmBytesOf(output, wasmCount(wasmJson)),
      engineMs: performance.now() - started,
      memoryBytes: memory?.buffer.byteLength ?? 0,
      tablesLoaded: library_tables(),
      ...(libraryStage === null ? {} : { libraryTables: libraryStage.used(reran) }),
    };
    output.free();
    reply({ kind: 'done', result });
  } catch (error) {
    const trap = trapKind(error);
    if (trap === null) {
      reply({ kind: 'failed', message: String(error) });
    } else {
      reply({ kind: 'trap', trap, message: String(error) });
    }
  }
}

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const request = event.data;
  if (request.kind === 'init') {
    init(request.module);
  } else if (request.kind === 'shards') {
    pending.get(request.request)?.(request.results);
    pending.delete(request.request);
  } else {
    void run(request.source, request.flags, request.refused, request.extras, request.wasm ?? [], request.libraries ?? null);
  }
};
