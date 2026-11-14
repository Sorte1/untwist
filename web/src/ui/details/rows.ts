import { explainLimit } from '../../limits';
import type { DiagnosticRow } from '../diagnostics-model';
import { moduleLabel, rangeLabel, type LibraryRow } from '../../libraries/libraries';
import type { Report, RunStats, StageRow } from './report';
import type { LibraryTablesUsed } from '../../engine/types';

export interface Pair {
  readonly label: string;
  readonly value: string;
}

export const NOT_RECORDED = 'not recorded';

export function ms(value: number): string {
  return value > 0 ? `${value.toFixed(value < 10 ? 2 : 0)} ms` : '—';
}

export function bytes(value: number): string {
  if (value < 1024) {
    return `${value} B`;
  }
  return value < 1024 * 1024 ? `${(value / 1024).toFixed(1)} KB` : `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

export function passesOf(stage: StageRow): Array<readonly [string, number, number]> {
  const timeOf = new Map(stage.per_pass_ms);
  return stage.per_pass.filter(([, changes]) => changes > 0).map(([name, changes]) => [name, changes, timeOf.get(name) ?? 0] as const);
}

export function detectionPairs(report: Report): Pair[] {
  const families = report.detection.families;
  return [
    { label: 'Bundle', value: report.detection.bundle ?? 'none detected' },
    { label: 'Modules', value: String(report.detection.modules) },
    {
      label: 'Obfuscator',
      value: families.length === 0 ? 'none detected' : [...new Set(families.map((row) => row.family))].join(', '),
    },
    ...families.map((row) => ({ label: `  ${row.pass}`, value: `${row.changes} changes` })),
  ];
}

export function libraryPairs(rows: readonly LibraryRow[]): Pair[] {
  if (rows.length === 0) {
    return [{ label: 'Libraries', value: 'none detected' }];
  }
  return rows.map((row) => ({ label: row.library, value: row.range === '' ? moduleLabel(row.modules) : `${rangeLabel(row)} · ${moduleLabel(row.modules)}` }));
}

export function tablePairs(used: LibraryTablesUsed | undefined): Pair[] {
  if (used === undefined || (used.fetched.length === 0 && used.failed.length === 0)) {
    return [];
  }
  const fetched = used.fetched.length === 0 ? [] : [{ label: 'Tables loaded', value: `${used.fetched.join(', ')}${used.reran ? ' (found in the output, run again)' : ''}` }];
  const failed = used.failed.length === 0 ? [] : [{ label: 'Tables not loaded', value: used.failed.join(', ') }];
  return [...fetched, ...failed, { label: 'Table scan / load', value: `${ms(used.scanMs)} / ${ms(used.loadMs)}` }];
}

export function limitPairs(report: Report): Pair[] {
  const { limits, engine } = report;
  const fired = limits.fired.map(explainLimit);
  return [
    { label: 'Fired', value: fired.length === 0 ? 'none' : fired.map((limit) => limit.name).join(', ') },
    ...fired.filter((limit) => limit.why !== '').map((limit) => ({ label: `  ${limit.name}`, value: limit.why })),
    { label: 'Fuel cap', value: limits.fuelCap >= Number.MAX_SAFE_INTEGER ? 'unlimited' : `${limits.fuelCap.toLocaleString('en-US')} steps per decode unit` },
    { label: 'Fuel used', value: fuelUsed(report.stats) },
    { label: 'Max iterations', value: limits.maxIterations === undefined ? 'until nothing changes' : String(limits.maxIterations) },
    { label: 'Nesting budget', value: String(limits.nestingBudget) },
    { label: 'Evaluator call depth cap', value: String(engine.callDepthCap) },
    { label: 'Evaluator nesting cap', value: String(engine.evaluatorNestingCap) },
    {
      label: 'Evaluator peak call depth',
      value: report.stats === undefined ? NOT_RECORDED : `${report.stats.decode.peakCallDepth.toLocaleString('en-US')} (cap ${engine.callDepthCap.toLocaleString('en-US')})`,
    },
  ];
}

const count = (value: number) => value.toLocaleString('en-US');

function fuelUsed(stats: RunStats | undefined): string {
  if (stats === undefined) {
    return NOT_RECORDED;
  }
  const { steps, peakUnitSteps, units } = stats.decode;
  if (units === 0) {
    return 'no decode unit ran';
  }
  return `${count(steps)} steps over ${count(units)} decode ${units === 1 ? 'unit' : 'units'}; at most ${count(peakUnitSteps)} in one`;
}

const COLLISION_LABELS: ReadonlyArray<readonly [keyof RunStats['collisions'], string]> = [
  ['boundInScope', 'already bound in scope'],
  ['freeReference', 'a free reference uses it'],
  ['label', 'a label uses it'],
  ['excluded', 'reserved or kept'],
];

function collisionPairs(stats: RunStats | undefined): Pair[] {
  if (stats === undefined) {
    return [{ label: 'Collision reasons', value: NOT_RECORDED }];
  }
  const total = COLLISION_LABELS.reduce((sum, [key]) => sum + stats.collisions[key], 0);
  if (total === 0) {
    return [{ label: 'Collision reasons', value: 'none' }];
  }
  return [
    { label: 'Collision reasons', value: `${count(total)} ${total === 1 ? 'name' : 'names'} skipped` },
    ...COLLISION_LABELS.filter(([key]) => stats.collisions[key] > 0).map(([key, label]) => ({ label: `  ${label}`, value: count(stats.collisions[key]) })),
  ];
}

export function warningClassPairs(rows: readonly DiagnosticRow[]): Pair[] {
  const counts = new Map<string, number>();
  for (const row of rows) {
    if (row.level === 'warning') {
      counts.set(row.pass, (counts.get(row.pass) ?? 0) + 1);
    }
  }
  if (counts.size === 0) {
    return [{ label: 'Warnings', value: 'none' }];
  }
  return [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).map(([pass, n]) => ({ label: pass, value: count(n) }));
}

export function renamePairs(report: Report): Pair[] {
  const stages = report.stages.filter((stage) => stage.stage === 'rename');
  const notes = report.notes.filter((note) => note.pass.startsWith('rename'));
  return [
    { label: 'Bindings renamed', value: stages.length === 0 ? 'rename was off' : String(stages.reduce((sum, stage) => sum + stage.changes, 0)) },
    ...notes.map((note) => ({ label: note.pass, value: note.message })),
    ...collisionPairs(report.stats),
  ];
}

export interface Resources {
  readonly workerMs: number;
  readonly engineMs: number;
  readonly memoryBytes: number;
}

export function resourcePairs(report: Report, run: Resources, stageMs: number): Pair[] {
  return [
    { label: 'Input', value: bytes(report.sizes.inputBytes) },
    { label: 'Output', value: bytes(report.sizes.outputBytes) },
    { label: 'Peak heap (engine)', value: report.limits.heapPeak > 0 ? bytes(report.limits.heapPeak) : NOT_RECORDED },
    { label: 'Wasm linear memory', value: bytes(run.memoryBytes) },
    { label: 'Worker wall time', value: ms(run.workerMs) },
    { label: 'Engine call', value: ms(run.engineMs) },
    { label: 'Stage total', value: ms(stageMs) },
    { label: 'Engine', value: `untwist ${report.engine.version} · build ${report.engine.build}` },
  ];
}
