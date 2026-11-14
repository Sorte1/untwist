import { readFileSync, writeFileSync } from "node:fs";
import { gunzipSync, gzipSync } from "node:zlib";
import { entries } from "../libcorpus/libs.mjs";

const [name, input, output] = process.argv.slice(2);
const cmp = (a, b) => {
  const x = a.split(".").map(Number);
  const y = b.split(".").map(Number);
  for (let i = 0; i < 3; i += 1) if (x[i] !== y[i]) return x[i] - y[i];
  return 0;
};
const versions = entries().filter((entry) => entry.name === name).map((entry) => entry.version).sort(cmp);
if (!versions.length) throw new Error(`no corpus versions for ${name}`);
const span = `${versions[0]}~${versions.at(-1)}`;
const lines = gunzipSync(readFileSync(input)).toString().split("\n");
const stringsAt = lines.findIndex((line) => line.startsWith("#strings\t"));
const count = Number(lines[stringsAt].split("\t")[1]);
const strings = lines.slice(stringsAt + 1, stringsAt + 1 + count);
let index = strings.indexOf(span);
if (index < 0) {
  index = count;
  lines.splice(stringsAt + 1 + count, 0, span);
  lines[stringsAt] = `#strings\t${count + 1}`;
}
const rowsAt = lines.findIndex((line) => line.startsWith("#rows\t"));
for (let i = rowsAt + 1; i < lines.length; i += 1) {
  if (!lines[i]) continue;
  const cells = lines[i].split("\t");
  cells[3] = String(index);
  lines[i] = cells.join("\t");
}
const head = lines.slice(0, rowsAt);
const rows = [...new Set(lines.slice(rowsAt + 1).filter(Boolean))];
writeFileSync(output, gzipSync(`${[...head, `#rows\t${rows.length}`, ...rows].join("\n")}\n`, { level: 9 }));
console.log(`${input}: every row now covers ${name} ${span}`);
