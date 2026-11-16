const vm = require("vm"), fs = require("fs");
const ctx = vm.createContext({ console });
for (const f of process.argv.slice(2)) vm.runInContext(fs.readFileSync(f, "utf8"), ctx, { filename: f });
