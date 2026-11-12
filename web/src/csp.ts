const HEADER = 'content-security-policy:';
const HEADER_ONLY = ['frame-ancestors', 'report-uri', 'sandbox'];

export function headerPolicy(headers: string): string {
  const line = headers
    .split('\n')
    .map((text) => text.trim())
    .find((text) => text.toLowerCase().startsWith(HEADER));
  if (line === undefined) {
    throw new Error('_headers has no Content-Security-Policy');
  }
  return line.slice(HEADER.length).trim();
}

export function metaPolicy(headers: string): string {
  return headerPolicy(headers)
    .split(';')
    .map((directive) => directive.trim())
    .filter((directive) => directive !== '' && !HEADER_ONLY.includes(directive.split(/\s+/)[0] ?? ''))
    .join('; ');
}
