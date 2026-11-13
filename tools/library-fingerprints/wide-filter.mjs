import { readFileSync, writeFileSync } from "node:fs";
import { gunzipSync, gzipSync } from "node:zlib";

const [denyFile, input, output] = process.argv.slice(2);
const deny = new Set(readFileSync(denyFile, "utf8").split("\n").filter(Boolean).map((line) => line.split("\t").slice(0, 2).join("\t")));
const text = gunzipSync(readFileSync(input)).toString();
const lines = text.split("\n");
const stringsAt = lines.findIndex((line) => line.startsWith("#strings\t"));
const strings = lines.slice(stringsAt + 1, stringsAt + 1 + Number(lines[stringsAt].split("\t")[1]));
const rowsAt = lines.findIndex((line) => line.startsWith("#rows\t"));
const rows = lines.slice(rowsAt + 1).filter(Boolean);
const kept = rows.filter((row) => {
  const cells = row.split("\t");
  return !deny.has(`${cells[1]}\t${strings[Number(cells[2])]}`) && !deny.has(`${cells[1]}\t*`);
});
const out = [...lines.slice(0, rowsAt), `#rows\t${kept.length}`, ...kept].join("\n");
writeFileSync(output, gzipSync(`${out}\n`, { level: 9 }));
console.log(`${input}: ${rows.length - kept.length} rows denied, ${kept.length} kept`);
