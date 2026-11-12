const IPV4 = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/;
const PLATFORM_HOSTS = ['168.63.129.16'];
const LOCAL_SUFFIXES = ['.localhost', '.local', '.internal', '.home.arpa'];
const NAT64_PREFIX = [0x64, 0xff9b, 0, 0, 0, 0];
const LOCAL_NAT64_PREFIX = [0x64, 0xff9b, 1];
const TRANSLATED_PREFIX = [0, 0, 0, 0, 0xffff, 0];
const MAPPED_PREFIX = [0, 0, 0, 0, 0, 0xffff];
const TEREDO_PREFIX = [0x2001, 0];
const SIX_TO_FOUR = 0x2002;

type Octets = readonly [number, number, number, number];

function startsWith(segments: readonly number[], prefix: readonly number[]): boolean {
  return prefix.every((value, index) => segments[index] === value);
}

function v4Octets(text: string): Octets | null {
  const match = IPV4.exec(text);
  if (match === null) {
    return null;
  }
  const octets = match.slice(1).map(Number);
  return octets.every((octet) => octet <= 255) ? [octets[0]!, octets[1]!, octets[2]!, octets[3]!] : null;
}

function v4Class([first, second, third, fourth]: Octets): string | null {
  if (first === 0) {
    return 'unspecified';
  }
  if (first === 127) {
    return 'loopback';
  }
  if (first === 169 && second === 254) {
    return 'link-local';
  }
  if (first === 10 || (first === 172 && second >= 16 && second < 32) || (first === 192 && second === 168)) {
    return 'private';
  }
  if (first === 100 && second >= 64 && second < 128) {
    return 'shared';
  }
  if (first >= 224 && first < 240) {
    return 'multicast';
  }
  const protocolAssignments = first === 192 && second === 0 && third === 0;
  const benchmarking = first === 198 && (second === 18 || second === 19);
  if (first >= 240 || protocolAssignments || benchmarking) {
    return 'reserved';
  }
  return PLATFORM_HOSTS.includes(`${first}.${second}.${third}.${fourth}`) ? 'platform' : null;
}

function v6Segments(text: string): number[] | null {
  const halves = text.split('::');
  if (halves.length > 2) {
    return null;
  }
  const groups = (half: string | undefined) => (half === undefined || half === '' ? [] : half.split(':'));
  const head = groups(halves[0]);
  const tail = groups(halves[1]);
  const missing = 8 - head.length - tail.length;
  if ((halves.length === 1 && missing !== 0) || missing < 0) {
    return null;
  }
  const all = [...head, ...Array<string>(halves.length === 2 ? missing : 0).fill('0'), ...tail];
  if (!all.every((group) => /^[0-9a-f]{1,4}$/i.test(group))) {
    return null;
  }
  return all.map((group) => parseInt(group, 16));
}

function embeddedV4(segments: readonly number[]): Octets | null {
  const from = (high: number, low: number): Octets => [high >> 8, high & 0xff, low >> 8, low & 0xff];
  if (startsWith(segments, MAPPED_PREFIX) || startsWith(segments, NAT64_PREFIX) || startsWith(segments, TRANSLATED_PREFIX)) {
    return from(segments[6]!, segments[7]!);
  }
  if (segments[0] === SIX_TO_FOUR) {
    return from(segments[1]!, segments[2]!);
  }
  if (startsWith(segments, [0, 0, 0, 0, 0, 0])) {
    return from(segments[6]!, segments[7]!);
  }
  return null;
}

function v6Class(segments: readonly number[]): string | null {
  if (segments.every((segment) => segment === 0)) {
    return 'unspecified';
  }
  if (startsWith(segments, [0, 0, 0, 0, 0, 0, 0, 1])) {
    return 'loopback';
  }
  const embedded = embeddedV4(segments);
  if (embedded !== null) {
    return v4Class(embedded);
  }
  const first = segments[0]!;
  if ((first & 0xfe00) === 0xfc00 || (first & 0xffc0) === 0xfec0 || startsWith(segments, LOCAL_NAT64_PREFIX)) {
    return 'private';
  }
  if (startsWith(segments, TEREDO_PREFIX)) {
    return 'reserved';
  }
  if ((first & 0xffc0) === 0xfe80) {
    return 'link-local';
  }
  return (first & 0xff00) === 0xff00 ? 'multicast' : null;
}

function nameRefusal(name: string): string | null {
  if (name === 'localhost' || name.endsWith('.localhost')) {
    return `the host ${name} is a loopback name`;
  }
  if (!name.includes('.') || LOCAL_SUFFIXES.some((suffix) => name.endsWith(suffix))) {
    return `the host ${name} is a local network name`;
  }
  return null;
}

export function hostRefusal(hostname: string): string | null {
  const name = hostname.toLowerCase().replace(/\.$/, '');
  if (name === '' || name.startsWith('.') || name.endsWith('.') || name.includes('..')) {
    return `the host ${hostname} has an empty label`;
  }
  if (name.startsWith('[') && name.endsWith(']')) {
    const segments = v6Segments(name.slice(1, -1));
    const kind = segments === null ? 'unreadable' : v6Class(segments);
    return kind === null ? null : `the host is the ${kind} address ${name}`;
  }
  const octets = v4Octets(name);
  if (octets !== null) {
    const kind = v4Class(octets);
    return kind === null ? null : `the host is the ${kind} address ${name}`;
  }
  return nameRefusal(name);
}
