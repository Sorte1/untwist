export type Phase =
  | 'tables'
  | 'start'
  | 'parse'
  | 'prepare'
  | 'deobfuscate'
  | 'unminify'
  | 'jsx'
  | 'cleanup'
  | 'rename'
  | 'modern'
  | 'library'
  | 'print'
  | 'unpack'
  | 'transfer'
  | 'editor';
export type StepState = 'done' | 'current' | 'pending';
export type Attempt = 'first' | 'retry' | 'rerun';

export interface StageMark {
  readonly name: string;
  readonly at: number;
}

export interface Step {
  readonly phase: Phase;
  readonly label: string;
  readonly state: StepState;
  readonly ms: number;
}

export interface RunProgress {
  readonly steps: readonly Step[];
  readonly stage: string | null;
  readonly round: number;
  readonly stageMs: number;
  readonly attempt: Attempt;
  readonly earlierMs: number;
}

export interface ProgressOptions {
  readonly libraries: boolean;
  readonly editor: boolean;
}

export const FOLDS_MARK = 'untwist:folds';

export const MARK = {
  scan: 'scan',
  tables: 'tables:',
  start: 'start',
  loadTables: 'load-tables',
  transfer: 'transfer',
  scanOutput: 'scan-output',
  retry: 'retry',
  rerun: 'rerun',
  editor: 'editor',
  folds: 'folds',
  end: 'end',
} as const;

const ORDER: readonly Phase[] = ['tables', 'start', 'parse', 'prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'rename', 'modern', 'library', 'print', 'unpack', 'transfer', 'editor'];

const LABELS: Readonly<Record<Phase, string>> = {
  tables: 'Library tables',
  start: 'Start worker',
  parse: 'Parse',
  prepare: 'Prepare',
  deobfuscate: 'Deobfuscate',
  unminify: 'Unminify',
  jsx: 'JSX',
  cleanup: 'Clean up',
  rename: 'Rename',
  modern: 'Modern',
  library: 'Library names',
  print: 'Print',
  unpack: 'Unpack',
  transfer: 'Transfer output',
  editor: 'Editor',
};

const STAGES: Readonly<Record<string, Phase>> = {
  [MARK.scan]: 'tables',
  [MARK.loadTables]: 'tables',
  [MARK.scanOutput]: 'tables',
  [MARK.start]: 'start',
  [MARK.retry]: 'start',
  [MARK.rerun]: 'start',
  parse: 'parse',
  prepare: 'prepare',
  layers: 'prepare',
  deobfuscate: 'deobfuscate',
  protections: 'deobfuscate',
  unminify: 'unminify',
  lexical: 'unminify',
  jsx: 'jsx',
  cleanup: 'cleanup',
  rename: 'rename',
  'rename-shorthand': 'rename',
  modern: 'modern',
  library: 'library',
  print: 'print',
  unpack: 'unpack',
  [MARK.transfer]: 'transfer',
  [MARK.editor]: 'editor',
  [MARK.folds]: 'editor',
};

const ENGINE_PHASES: ReadonlySet<Phase> = new Set(['start', 'parse', 'prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'rename', 'modern', 'library', 'print', 'unpack', 'transfer']);

export function phaseOf(stage: string): Phase | null {
  if (stage.startsWith(MARK.tables)) {
    return 'tables';
  }
  return STAGES[stage] ?? null;
}

function words(flags: string): string[] {
  return flags.split(' ').filter((word) => word !== '');
}

function renames(list: readonly string[]): boolean {
  const at = list.indexOf('--rename');
  return at !== -1 && list[at + 1] !== 'none';
}

export function disabledPasses(flags: string): string[] {
  const list = words(flags);
  return list.flatMap((word, at) => (word === '--disable' ? (list[at + 1] ?? '').split(',') : [])).filter((name) => name !== '');
}

export function librariesOn(flags: string): boolean {
  return !disabledPasses(flags).includes('library-names');
}

export function expectedPhases(flags: string, options: ProgressOptions = { libraries: librariesOn(flags), editor: false }): Phase[] {
  const list = words(flags);
  const off = (flag: string) => list.includes(flag);
  const shown: Readonly<Record<Phase, boolean>> = {
    tables: options.libraries,
    start: true,
    parse: true,
    prepare: true,
    deobfuscate: !off('--no-deobfuscate'),
    unminify: !off('--no-unminify'),
    jsx: !off('--no-jsx'),
    cleanup: !off('--no-deobfuscate'),
    rename: renames(list),
    modern: false,
    library: options.libraries,
    print: true,
    unpack: !off('--no-unpack'),
    transfer: true,
    editor: options.editor,
  };
  return ORDER.filter((phase) => shown[phase]);
}

interface Span {
  readonly mark: StageMark;
  readonly phase: Phase | null;
  readonly ms: number;
}

function spansOf(marks: readonly StageMark[], now: number): Span[] {
  const sorted = [...marks].sort((a, b) => a.at - b.at);
  let phase: Phase | null = null;
  return sorted.map((mark, at) => {
    phase = phaseOf(mark.name) ?? phase;
    const next = sorted[at + 1]?.at ?? now;
    return { mark, phase, ms: Math.max(0, next - mark.at) };
  });
}

function lastBoundary(spans: readonly Span[]): number {
  return spans.findLastIndex((span) => span.mark.name === MARK.retry || span.mark.name === MARK.rerun);
}

function attemptOf(spans: readonly Span[], boundary: number): Attempt {
  const name = spans[boundary]?.mark.name;
  return name === MARK.retry ? 'retry' : name === MARK.rerun ? 'rerun' : 'first';
}

function counted(spans: readonly Span[], boundary: number): Span[] {
  return spans.filter((span, at) => span.phase !== null && (at >= boundary || !ENGINE_PHASES.has(span.phase)));
}

function earlierMs(spans: readonly Span[], boundary: number): number {
  return spans.slice(0, Math.max(0, boundary)).filter((span) => span.phase !== null && ENGINE_PHASES.has(span.phase)).reduce((sum, span) => sum + span.ms, 0);
}

function stepsOf(flags: string, options: ProgressOptions, spans: readonly Span[], current: Phase | null): Step[] {
  const totals = new Map<Phase, number>();
  for (const span of spans) {
    if (span.phase !== null && span.mark.name !== MARK.end) {
      totals.set(span.phase, (totals.get(span.phase) ?? 0) + span.ms);
    }
  }
  const expected = new Set(expectedPhases(flags, options));
  return ORDER.filter((phase) => expected.has(phase) || totals.has(phase)).map((phase) => ({
    phase,
    label: LABELS[phase],
    state: phase === current ? 'current' : totals.has(phase) ? 'done' : 'pending',
    ms: totals.get(phase) ?? 0,
  }));
}

export function progressOf(flags: string, marks: readonly StageMark[], now: number, options: ProgressOptions = { libraries: librariesOn(flags), editor: false }): RunProgress {
  const spans = spansOf(marks, now);
  const boundary = lastBoundary(spans);
  const scope = counted(spans, Math.max(0, boundary));
  const last = spans.at(-1);
  const ended = last?.mark.name === MARK.end;
  const current = ended ? null : (last?.phase ?? null);
  const stage = ended || last === undefined ? null : last.mark.name;
  return {
    steps: stepsOf(flags, options, scope, current),
    stage,
    round: stage === null ? 0 : spans.filter((span, at) => at > boundary && span.mark.name === stage).length || 1,
    stageMs: ended || last === undefined ? 0 : last.ms,
    attempt: attemptOf(spans, boundary),
    earlierMs: earlierMs(spans, boundary),
  };
}

export function tablesMark(tables: readonly { readonly label: string; readonly bytes: number }[], size: (bytes: number) => string): string {
  return `${MARK.tables}${tables.map((table) => `${table.label} (${size(table.bytes)})`).join(', ')}`;
}

const LINES: Readonly<Record<string, string>> = {
  [MARK.scan]: 'Scanning the input for library code…',
  [MARK.start]: 'Starting the engine…',
  [MARK.retry]: 'Starting the engine…',
  [MARK.rerun]: 'Starting the engine…',
  [MARK.loadTables]: 'Loading library tables into the engine…',
  [MARK.transfer]: 'Transferring the output…',
  [MARK.scanOutput]: 'Scanning the output for library code…',
  [MARK.editor]: 'Loading the output into the editor…',
  [MARK.folds]: 'Folding library code…',
};

export function stageLine(progress: RunProgress): string {
  const stage = progress.stage;
  if (stage === null) {
    return 'Starting the engine…';
  }
  if (stage.startsWith(MARK.tables)) {
    return `Fetching library tables: ${stage.slice(MARK.tables.length)}…`;
  }
  const line = LINES[stage];
  if (line !== undefined) {
    return line;
  }
  return progress.round > 1 ? `Stage: ${stage} · round ${progress.round}` : `Stage: ${stage}`;
}

export function attemptLine(progress: RunProgress): string {
  if (progress.attempt === 'retry') {
    return 'Retrying (depth limit): the first attempt hit the stack limit';
  }
  if (progress.attempt === 'rerun') {
    return 'Running again with the library tables found in the output';
  }
  return '';
}

export function durationLabel(ms: number): string {
  return ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(2)} s`;
}

export interface TimelineRow {
  readonly label: string;
  readonly value: string;
}

export function timelineRows(progress: RunProgress): TimelineRow[] {
  const rows = progress.steps.filter((step) => step.state !== 'pending').map((step) => ({ label: step.label, value: durationLabel(step.ms) }));
  if (progress.attempt === 'first' || progress.earlierMs <= 0) {
    return rows;
  }
  const label = progress.attempt === 'retry' ? 'Attempt that hit the depth limit' : 'Run before the library tables';
  return [{ label, value: durationLabel(progress.earlierMs) }, ...rows];
}

export function timelineText(progress: RunProgress): string {
  return timelineRows(progress).map((row) => `${row.label}: ${row.value}`).join('\n');
}
