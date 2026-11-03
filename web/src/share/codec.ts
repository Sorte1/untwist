import { deflateSync, Inflate } from 'fflate';
import { LINK_CODE_CAP } from './link';

export const SHARED_TEXT_CAP = 4 * 1024 * 1024;
export const PACKED_CAP = 2 * LINK_CODE_CAP;

const CHUNK = 0x8000;
const INFLATE_SLICE = 4096;
const UNREADABLE = 'The code in the link could not be read.';
const BASE64URL = /^[A-Za-z0-9_-]+$/;

class LinkTooLarge extends Error {
  constructor(message = `The code in the link is larger than ${SHARED_TEXT_CAP / (1024 * 1024)} MB.`) {
    super(message);
  }
}

function toBase64Url(bytes: Uint8Array): string {
  let binary = '';
  for (let at = 0; at < bytes.length; at += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(at, at + CHUNK));
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromBase64Url(text: string): Uint8Array {
  if (!BASE64URL.test(text) || text.length % 4 === 1) {
    throw new Error(UNREADABLE);
  }
  const binary = atob(text.replace(/-/g, '+').replace(/_/g, '/'));
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

function inflated(bytes: Uint8Array): Uint8Array {
  const chunks: Uint8Array[] = [];
  let total = 0;
  let ended = false;
  const stream = new Inflate((chunk, final) => {
    total += chunk.length;
    if (total > SHARED_TEXT_CAP) {
      throw new LinkTooLarge();
    }
    chunks.push(chunk);
    ended = final;
  });
  for (let at = 0; at < bytes.length; at += INFLATE_SLICE) {
    stream.push(bytes.subarray(at, at + INFLATE_SLICE), at + INFLATE_SLICE >= bytes.length);
  }
  if (!ended) {
    throw new Error(UNREADABLE);
  }
  const out = new Uint8Array(total);
  chunks.reduce((at, chunk) => {
    out.set(chunk, at);
    return at + chunk.length;
  }, 0);
  return out;
}

export function packCode(text: string): string {
  return toBase64Url(deflateSync(new TextEncoder().encode(text), { level: 9 }));
}

export function unpackCode(packed: string): string {
  if (packed.length > PACKED_CAP) {
    throw new LinkTooLarge('The code in the link is too long to open.');
  }
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(inflated(fromBase64Url(packed)));
  } catch (error) {
    throw error instanceof LinkTooLarge ? error : new Error(UNREADABLE);
  }
}
