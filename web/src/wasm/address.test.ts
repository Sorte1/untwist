import { describe, expect, it } from 'vitest';
import { hostRefusal } from './address';

function kind(host: string): string | null {
  const refusal = hostRefusal(host);
  return refusal === null ? null : (/the (?:host is the )?(\S+) address/.exec(refusal)?.[1] ?? refusal);
}

describe('host refusals', () => {
  it('lets public addresses and names through', () => {
    for (const host of ['1.1.1.1', '93.184.216.34', '172.32.0.1', '100.128.0.1', '192.0.1.1', '198.17.255.255', '198.20.0.1', '168.63.129.17', '[2606:4700:4700::1111]', '[2001:4860:4860::8888]', '[::ffff:808:808]', '[64:ff9b::808:808]', 'cdn.dev', 'wasm.example.test']) {
      expect(hostRefusal(host), host).toBeNull();
    }
  });

  it('refuses the address classes the cli refuses', () => {
    const cases: Array<[string, string]> = [
      ['127.0.0.1', 'loopback'], ['127.255.255.254', 'loopback'], ['[::1]', 'loopback'],
      ['169.254.169.254', 'link-local'], ['[fe80::1]', 'link-local'], ['[febf::1]', 'link-local'],
      ['10.0.0.1', 'private'], ['172.16.0.1', 'private'], ['172.31.255.255', 'private'], ['192.168.1.1', 'private'],
      ['[fc00::1]', 'private'], ['[fd12:3456::1]', 'private'], ['[fec0::1]', 'private'], ['[64:ff9b:1::a00:1]', 'private'],
      ['0.0.0.0', 'unspecified'], ['0.1.2.3', 'unspecified'], ['[::]', 'unspecified'],
      ['224.0.0.1', 'multicast'], ['[ff02::1]', 'multicast'],
      ['100.64.0.1', 'shared'], ['100.127.255.255', 'shared'],
      ['240.0.0.1', 'reserved'], ['255.255.255.255', 'reserved'], ['192.0.0.192', 'reserved'], ['198.18.0.1', 'reserved'],
      ['198.19.255.255', 'reserved'], ['[2001:0:4136:e378:8000:63bf:3fff:fdd2]', 'reserved'],
      ['168.63.129.16', 'platform'], ['[::ffff:a83f:8110]', 'platform'],
    ];
    for (const [host, expected] of cases) {
      expect(kind(host), host).toBe(expected);
    }
  });

  it('judges an ipv4 address inside an ipv6 one as the ipv4 address', () => {
    expect(kind('[::ffff:7f00:1]')).toBe('loopback');
    expect(kind('[::ffff:a00:1]')).toBe('private');
    expect(kind('[64:ff9b::7f00:1]')).toBe('loopback');
    expect(kind('[2002:c0a8:101::1]')).toBe('private');
    expect(kind('[::a00:1]')).toBe('private');
    expect(kind('[::ffff:0:7f00:1]')).toBe('loopback');
  });

  it('refuses a host with an empty label', () => {
    for (const host of ['127.0.0.1..', 'localhost..', 'a..b.dev', '.cdn.dev', '']) {
      expect(hostRefusal(host), host).toContain('empty label');
    }
    expect(hostRefusal('cdn.dev.')).toBeNull();
  });

  it('refuses loopback and local network names it cannot resolve', () => {
    for (const host of ['localhost', 'localhost.', 'app.localhost', 'printer.local', 'db.internal', 'nas.home.arpa', 'intranet']) {
      expect(hostRefusal(host), host).not.toBeNull();
    }
  });
});
