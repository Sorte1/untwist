import { analysis_cap_bytes, analyze, initSync, type Analysis } from '../engine/pkg/untwist_wasm.js';
import type { LangReply, LangRequest, QueryOp } from './client';

let memory: WebAssembly.Memory | null = null;
let current: Analysis | null = null;
let lastClean: Analysis | null = null;
let currentGen = -1;

function reply(message: LangReply): void {
  self.postMessage(message);
}

function init(module: WebAssembly.Module): void {
  try {
    memory = initSync({ module }).memory;
    reply({ kind: 'ready', capBytes: analysis_cap_bytes() });
  } catch (error) {
    reply({ kind: 'failed', message: String(error) });
  }
}

function keep(next: Analysis, clean: boolean): void {
  if (clean) {
    if (lastClean !== current) {
      lastClean?.free();
    }
    if (current !== null && current !== next) {
      current.free();
    }
    lastClean = next;
  } else if (current !== null && current !== lastClean) {
    current.free();
  }
  current = next;
}

function analyzeText(gen: number, text: string): void {
  const started = performance.now();
  const next = analyze(text);
  const problems = next.problems_json();
  keep(next, problems === '[]');
  currentGen = gen;
  reply({
    kind: 'analyzed',
    gen,
    available: next.available(),
    problems,
    ms: performance.now() - started,
    memoryBytes: memory?.buffer.byteLength ?? 0,
  });
}

function answer(analysis: Analysis, op: QueryOp, pos: number, name: string): unknown {
  switch (op) {
    case 'definition':
      return analysis.definition(pos) ?? null;
    case 'references':
      return analysis.references(pos);
    case 'rename':
      return analysis.rename(pos, name);
    case 'completions':
      return analysis.completions(pos);
  }
}

function exportSpan(text: string, name: string): Uint32Array {
  const other = analyze(text);
  try {
    return other.export_span(name);
  } finally {
    other.free();
  }
}

function handle(request: LangRequest): void {
  if (request.kind === 'init') {
    init(request.module);
  } else if (request.kind === 'analyze') {
    analyzeText(request.gen, request.text);
  } else if (request.kind === 'query') {
    const source = request.op === 'completions' && current !== lastClean ? (lastClean ?? current) : current;
    const value = source === null ? null : answer(source, request.op, request.pos, request.name);
    reply({ kind: 'answer', id: request.id, gen: currentGen, value });
  } else {
    reply({ kind: 'answer', id: request.id, gen: currentGen, value: exportSpan(request.text, request.name) });
  }
}

self.onmessage = (event: MessageEvent<LangRequest>) => {
  try {
    handle(event.data);
  } catch (error) {
    const request = event.data;
    if (request.kind === 'query' || request.kind === 'export') {
      reply({ kind: 'answer', id: request.id, gen: -1, value: null });
    } else {
      reply({ kind: 'failed', message: String(error) });
    }
  }
};
