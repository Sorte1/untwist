import type { EngineInfo } from './report';

export interface DebugBundle {
  readonly settings: { readonly flags: string };
  readonly report: unknown;
  readonly engine: EngineInfo;
}

export function downloadText(name: string, data: BlobPart, type: string): void {
  const url = URL.createObjectURL(new Blob([data], { type }));
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function debugBundle(flags: string, reportJson: string, engine: EngineInfo): string {
  const bundle: DebugBundle = { settings: { flags }, report: JSON.parse(reportJson) as unknown, engine };
  return JSON.stringify(bundle, null, 2);
}
