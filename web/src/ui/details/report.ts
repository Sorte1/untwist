import { reportValue } from '../../report-json';

export type IterationMs = readonly [number, number | null, number | null];

export interface StageRow {
  readonly stage: string;
  readonly round: number;
  readonly iterations: number;
  readonly changes: number;
  readonly per_pass: ReadonlyArray<readonly [string, number]>;
  readonly per_pass_ms: ReadonlyArray<readonly [string, number]>;
  readonly fixpoint_per_pass: ReadonlyArray<readonly [string, number]>;
  readonly fixpoint_per_pass_ms: ReadonlyArray<readonly [string, number]>;
  readonly iterations_ms: readonly IterationMs[];
  readonly elapsed_ms: number;
  readonly stop: string;
}

export interface NoteRow {
  readonly level: 'warning' | 'info' | 'error';
  readonly pass: string;
  readonly kind: 'refusal' | 'progress';
  readonly message: string;
  readonly inputSpan?: readonly [number, number];
}

export interface EngineInfo {
  readonly version: string;
  readonly build: string;
  readonly nestingBudget: number;
  readonly shadowStackBytes: number;
  readonly callDepthCap: number;
  readonly evaluatorNestingCap: number;
  readonly stepBudget: number;
}

export interface Report {
  readonly schema: string;
  readonly engine: EngineInfo;
  readonly stages: readonly StageRow[];
  readonly notes: readonly NoteRow[];
  readonly limits: {
    readonly fired: readonly string[];
    readonly nestingBudget: number;
    readonly fuelCap: number;
    readonly maxIterations?: number;
    readonly heapPeak: number;
  };
  readonly detection: {
    readonly bundle: string | null;
    readonly modules: number;
    readonly families: ReadonlyArray<{ readonly family: string; readonly pass: string; readonly changes: number }>;
  };
  readonly sizes: { readonly inputBytes: number; readonly outputBytes: number };
  readonly stats?: RunStats;
}

export interface RunStats {
  readonly decode: { readonly steps: number; readonly peakUnitSteps: number; readonly units: number; readonly peakCallDepth: number };
  readonly collisions: { readonly boundInScope: number; readonly freeReference: number; readonly label: number; readonly excluded: number };
}

export function parseReport(json: string): Report | null {
  const value = reportValue(json);
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const report = value as Partial<Report>;
  return Array.isArray(report.stages) && report.engine !== undefined ? (report as Report) : null;
}

export function stageTotalMs(report: Report): number {
  return report.stages.reduce((sum, stage) => sum + stage.elapsed_ms, 0);
}
