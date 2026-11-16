import fs from 'fs';
import vm from 'vm';
const rows = fs.readFileSync(process.argv[2], 'utf8').trim().split('\n');
const out = [];
for (const src of rows) {
  let s = 1, m = 1;
  try { new vm.Script(src); } catch (e) { s = e instanceof SyntaxError ? 0 : 2; }
  try { new vm.SourceTextModule(src); } catch (e) { m = e instanceof SyntaxError ? 0 : 2; }
  out.push(`${s}\t${m}\t${src}`);
}
fs.writeFileSync(process.argv[3], out.join('\n') + '\n');
