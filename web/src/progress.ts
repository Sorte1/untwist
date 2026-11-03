export type Phase = 'prepare' | 'deobfuscate' | 'unminify' | 'jsx' | 'cleanup' | 'rename' | 'unpack';
export type StepState = 'done' | 'current' | 'pending';

export interface StageMark {
  readonly name: string;
  readonly at: number;
}

export interface Step {
  readonly phase: Phase;
  readonly label: string;
  readonly state: StepState;
}

export interface RunProgress {
  readonly steps: readonly Step[];
  readonly stage: string | null;
  readonly round: number;
  readonly stageMs: number;
}

const ORDER: readonly Phase[] = ['prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'rename', 'unpack'];

const LABELS: Readonly<Record<Phase, string>> = {
  prepare: 'Prepare',
  deobfuscate: 'Deobfuscate',
  unminify: 'Unminify',
  jsx: 'JSX',
  cleanup: 'Clean up',
  rename: 'Rename',
  unpack: 'Unpack',
};

const STAGES: Readonly<Record<string, Phase>> = {
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
  unpack: 'unpack',
};

export function phaseOf(stage: string): Phase | null {
  return STAGES[stage] ?? null;
}

function renames(words: readonly string[]): boolean {
  const at = words.indexOf('--rename');
  return at !== -1 && words[at + 1] !== 'none';
}

export function expectedPhases(flags: string): Phase[] {
  const words = flags.split(' ').filter((word) => word !== '');
  const off = (flag: string) => words.includes(flag);
  const shown: Readonly<Record<Phase, boolean>> = {
    prepare: true,
    deobfuscate: !off('--no-deobfuscate'),
    unminify: !off('--no-unminify'),
    jsx: !off('--no-jsx'),
    cleanup: !off('--no-deobfuscate'),
    rename: renames(words),
    unpack: !off('--no-unpack'),
  };
  return ORDER.filter((phase) => shown[phase]);
}

export function progressOf(flags: string, marks: readonly StageMark[], now: number): RunProgress {
  const known = marks.filter((mark) => phaseOf(mark.name) !== null);
  const last = known.at(-1);
  const current = last === undefined ? null : phaseOf(last.name);
  const seen = new Set(known.map((mark) => phaseOf(mark.name)));
  const steps = expectedPhases(flags).map((phase) => ({
    phase,
    label: LABELS[phase],
    state: phase === current ? 'current' : seen.has(phase) ? 'done' : 'pending',
  }) satisfies Step);
  return {
    steps,
    stage: last?.name ?? null,
    round: last === undefined ? 0 : known.filter((mark) => mark.name === last.name).length,
    stageMs: last === undefined ? 0 : Math.max(0, now - last.at),
  };
}

export function stageLine(progress: RunProgress): string {
  if (progress.stage === null) {
    return 'Starting the engine…';
  }
  return progress.round > 1 ? `Stage: ${progress.stage} · round ${progress.round}` : `Stage: ${progress.stage}`;
}
