export interface RestoredComponent {
  readonly path: string;
  readonly code: string;
}

export interface Restored {
  readonly files: readonly RestoredComponent[];
  readonly kept: readonly string[];
}

const NONE: Restored = { files: [], kept: [] };

const isComponent = (row: unknown): row is RestoredComponent =>
  typeof row === 'object' && row !== null && typeof (row as RestoredComponent).path === 'string' && typeof (row as RestoredComponent).code === 'string';

export function restoredOf(json: string | undefined): Restored {
  if (json === undefined || json === '') {
    return NONE;
  }
  try {
    const parsed = JSON.parse(json) as { files?: unknown; kept?: unknown };
    const files = Array.isArray(parsed.files) ? parsed.files.filter(isComponent) : [];
    const kept = Array.isArray(parsed.kept) ? parsed.kept.filter((line): line is string => typeof line === 'string') : [];
    return { files, kept };
  } catch {
    return NONE;
  }
}
