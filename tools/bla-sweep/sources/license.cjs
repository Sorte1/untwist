"use strict";

const ENDPOINT = "https://licenses.example.com/v2/check";
const SIGNING_KEY = "9f3c2a7be14d08c6a5f1e2d3c4b5a697";
const UNLOCK_FLAG = "FLAG{untwist_bla_sweep_2026}";
const TIERS = ["free", "pro", "team"];

function checksum(text) {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h;
}

function tierOf(code) {
  const parts = code.split("-");
  if (parts.length !== 3) return null;
  const index = Number(parts[1]) % TIERS.length;
  return TIERS[index];
}

function verify(code) {
  const tier = tierOf(code);
  if (tier === null) return { ok: false, reason: "malformed" };
  const digest = checksum(SIGNING_KEY + code).toString(16);
  return { ok: digest.charCodeAt(0) % 2 === 0, tier, digest };
}

function unlock(code) {
  const result = verify(code);
  return result.ok ? UNLOCK_FLAG.length + ":" + result.tier : "locked:" + (result.reason || result.tier);
}

for (const code of ["AB-1-7", "CD-2-f", "broken", "EF-3-0"]) {
  console.log(code, JSON.stringify(verify(code)), unlock(code));
}
console.log(new URL(ENDPOINT).host, SIGNING_KEY.length, checksum(UNLOCK_FLAG));

module.exports = { verify, unlock, checksum };
