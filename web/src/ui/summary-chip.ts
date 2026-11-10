import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { reportField } from '../report-json';

interface Summary {
  readonly stages: number;
  readonly rounds: number;
  readonly totalMs: number;
}

export function summaryOf(json: string): Summary | null {
  const stages = reportField(json, 'stages') as ReadonlyArray<{ iterations: number; elapsed_ms: number }> | undefined;
  if (!Array.isArray(stages)) {
    return null;
  }
  return {
    stages: stages.length,
    rounds: stages.reduce((sum, stage) => sum + stage.iterations, 0),
    totalMs: stages.reduce((sum, stage) => sum + stage.elapsed_ms, 0),
  };
}

export function chipLabel(summary: Summary): string {
  const plural = (count: number, word: string) => `${count} ${word}${count === 1 ? '' : 's'}`;
  return `${plural(summary.stages, 'stage')} · ${plural(summary.rounds, 'round')} · ${(summary.totalMs / 1000).toFixed(2)} s`;
}

export function mountSummaryChip(store: Store<AppState>): void {
  const chip = byId('summary-chip');
  store.subscribe((state, previous) => {
    if (state.run.result === previous.run.result && state.run.status === previous.run.status) {
      return;
    }
    performance.mark('untwist:chip-start');
    const finished = state.run.status === 'done' || state.run.status === 'limit';
    const summary = finished && state.run.result !== null ? summaryOf(state.run.result.report) : null;
    chip.hidden = summary === null || summary.stages === 0;
    setText(chip, summary === null ? '' : chipLabel(summary));
    performance.measure('untwist:chip', 'untwist:chip-start');
  });
}
