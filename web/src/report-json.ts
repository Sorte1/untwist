const KEPT = 4;

interface Parsed {
  readonly json: string;
  readonly value: unknown;
}

let recent: readonly Parsed[] = [];

function parsed(json: string): unknown {
  try {
    return JSON.parse(json) as unknown;
  } catch {
    return undefined;
  }
}

export function reportValue(json: string): unknown {
  const hit = recent.find((entry) => entry.json === json);
  if (hit !== undefined) {
    return hit.value;
  }
  const entry = { json, value: parsed(json) };
  recent = [entry, ...recent.slice(0, KEPT - 1)];
  return entry.value;
}

export function reportField(json: string, key: string): unknown {
  const value = reportValue(json);
  return typeof value === 'object' && value !== null && !Array.isArray(value) && Object.hasOwn(value, key) ? (value as Record<string, unknown>)[key] : undefined;
}
