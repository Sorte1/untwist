import type { Settings } from './settings';

export type Mode = 'safe' | 'unsafe';

export const MODES: readonly Mode[] = ['safe', 'unsafe'];

export const UNSAFE_WARNING = 'may change behaviour in edge cases; assumes built-ins are not modified';

export const JSX_HINTS = {
  active: 'Turn createElement calls back into JSX',
  inactive: 'Inactive in Safe mode; needs Unsafe',
} as const;

const LABELS: Readonly<Record<Mode, string>> = { safe: 'Safe', unsafe: 'Unsafe' };

export function modeOf(settings: Settings): Mode {
  return settings.unsafe ? 'unsafe' : 'safe';
}

export function modeFromFlags(flags: string | null): Mode | null {
  if (flags === null) {
    return null;
  }
  return flags.split(' ').includes('--unsafe') ? 'unsafe' : 'safe';
}

export function modeLabel(mode: Mode | null): string {
  return mode === null ? '—' : LABELS[mode];
}

export function steppedMode(current: Mode, key: string): Mode | null {
  switch (key) {
    case 'Home':
      return 'safe';
    case 'End':
      return 'unsafe';
    case 'ArrowLeft':
    case 'ArrowRight':
    case 'ArrowUp':
    case 'ArrowDown':
      return current === 'safe' ? 'unsafe' : 'safe';
    default:
      return null;
  }
}

export function jsxInactive(settings: Settings): boolean {
  return settings.jsx && !settings.unsafe;
}
