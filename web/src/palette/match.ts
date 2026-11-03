export interface Command {
  readonly group: string;
  readonly title: string;
  readonly hint: string;
  readonly run: () => void;
}

export const RESULT_LIMIT = 200;

const WORD_START = 3;
const IN_RUN = 2;
const WHOLE = 20;
const WHOLE_WORD = 10;

function startsWord(text: string, at: number): boolean {
  return at === 0 || !/[a-z0-9]/.test(text[at - 1] ?? '');
}

function scoreFrom(query: string, text: string, start: number): number | null {
  let total = 0;
  let previous = -2;
  let at = start;
  for (const char of query) {
    const found = text.indexOf(char, at);
    if (found === -1) {
      return null;
    }
    total += 1 + (startsWord(text, found) ? WORD_START : 0) + (found === previous + 1 ? IN_RUN : 0);
    previous = found;
    at = found + 1;
  }
  return total;
}

export function score(query: string, text: string): number | null {
  const needle = query.toLowerCase().replace(/\s+/g, '');
  if (needle === '') {
    return 0;
  }
  const haystack = text.toLowerCase();
  const whole = haystack.indexOf(needle);
  const bonus = whole === -1 ? 0 : WHOLE + (startsWord(haystack, whole) ? WHOLE_WORD : 0);
  let best: number | null = null;
  for (let start = haystack.indexOf(needle[0] ?? ''); start !== -1; start = haystack.indexOf(needle[0] ?? '', start + 1)) {
    const found = scoreFrom(needle, haystack, start);
    if (found !== null && (best === null || found > best)) {
      best = found;
    }
  }
  return best === null ? null : best + bonus;
}

export function filterCommands<T extends Command>(commands: readonly T[], query: string, limit = RESULT_LIMIT): T[] {
  return commands
    .map((command, index) => ({ command, index, score: score(query, `${command.group} ${command.title}`) }))
    .filter((item): item is { command: T; index: number; score: number } => item.score !== null)
    .sort((a, b) => b.score - a.score || a.index - b.index)
    .slice(0, limit)
    .map((item) => item.command);
}
