import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";

const [, , file] = process.argv;
const src = readFileSync(file, "utf8");

function slice(startMarker, endMarker) {
  const start = src.indexOf(startMarker);
  if (start < 0) throw new Error(`missing ${startMarker}`);
  const end = src.indexOf(endMarker, start);
  if (end < 0) throw new Error(`missing ${endMarker}`);
  return src.slice(start, end + endMarker.length);
}

const arrayFn = slice("function iïîïïíî(){", "return iïîïïíî();}");
const decoderFn = slice("function iíìîiîï(", "iíìîiîï(iïïîïîî,iiîìíîì);}");
const rotator = slice("function(IÍîîîíi,IÎîìiîî){", "}(iïîïïíî,0x34d0e)");

const run = new Function(
  `${arrayFn}\n${decoderFn}\nconst before = iïîïïíî().slice();\n(${rotator});\nreturn { before, after: iïîïïíî() };`,
);
const { before, after } = run();
const n = before.length;
if (after.length !== n) throw new Error("length changed");

let rotation = -1;
for (let k = 0; k < n; k++) {
  if (before[k] !== after[0]) continue;
  let ok = true;
  for (let i = 0; i < n; i++) {
    if (after[i] !== before[(i + k) % n]) { ok = false; break; }
  }
  if (ok) { rotation = k; break; }
}

const hash = createHash("sha256");
let units = 0;
for (const s of after) {
  if (typeof s !== "string") throw new Error(`non-string element: ${typeof s}`);
  const buf = Buffer.alloc(4 + 2 * s.length);
  buf.writeUInt32LE(s.length, 0);
  for (let i = 0; i < s.length; i++) buf.writeUInt16LE(s.charCodeAt(i), 4 + 2 * i);
  hash.update(buf);
  units += s.length;
}
console.log(JSON.stringify({ length: n, rotation, units, sha256: hash.digest("hex") }));
