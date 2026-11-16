const fs = require('fs');
const lines = fs.readFileSync(process.argv[2], 'utf8').trim().split('\n');
const out = [];
for (const line of lines) {
  const { p, f } = JSON.parse(line);
  let ok = true;
  try { new Function('/' + p + '/' + f); } catch (e) { ok = false; }
  out.push(`${ok ? 1 : 0}\t${f}\t${p}`);
}
fs.writeFileSync(process.argv[3], out.join('\n') + '\n');
