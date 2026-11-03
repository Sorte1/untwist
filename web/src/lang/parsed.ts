export function parsed<T>(value: unknown): T | null {
  if (typeof value !== 'string') {
    return null;
  }
  try {
    return JSON.parse(value) as T;
  } catch {
    return null;
  }
}
