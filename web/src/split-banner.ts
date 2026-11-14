export const SPLIT_TEXT = "This bundle can be split with 'Assume ES modules'.";

export function splitsWhenAssumed(report: string): boolean {
  try {
    return (JSON.parse(report) as { detection?: { assumedSplit?: boolean } }).detection?.assumedSplit === true;
  } catch {
    return false;
  }
}
