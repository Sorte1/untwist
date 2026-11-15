const vm = require("vm"), fs = require("fs"), path = require("path");
const [entry, ...scripts] = process.argv.slice(2).filter((arg) => arg !== "--");
if (entry !== "-") require(path.resolve(entry));
for (const s of scripts) vm.runInThisContext(fs.readFileSync(s, "utf8"));
