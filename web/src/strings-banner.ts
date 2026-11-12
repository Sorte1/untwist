import { modeFromFlags } from './settings/mode';
import { reportField } from './report-json';

export const BANNER_TEXT = 'Strings were not decoded in Safe mode. Unsafe mode decodes them.';

const MACHINERY_PASS = 'remove-decoder-machinery';
const UNDECODED = /^decoder machinery kept: (\d+) reference\(s\) to the string array or its decoders did not decode/;

function notesOf(report: string): ReadonlyArray<{ readonly pass?: unknown; readonly message?: unknown }> {
  const notes = reportField(report, 'notes');
  return Array.isArray(notes) ? notes.filter((note): note is { readonly pass?: unknown; readonly message?: unknown } => typeof note === 'object' && note !== null) : [];
}

export function undecodedInSafe(report: string, outputFlags: string | null): number | null {
  if (modeFromFlags(outputFlags) !== 'safe') {
    return null;
  }
  let most: number | null = null;
  for (const note of notesOf(report)) {
    if (note.pass !== MACHINERY_PASS || typeof note.message !== 'string') {
      continue;
    }
    const match = UNDECODED.exec(note.message);
    if (match !== null) {
      most = Math.max(most ?? 0, Number(match[1]));
    }
  }
  return most;
}
