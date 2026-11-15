const vm = require("vm"), fs = require("fs");
globalThis.peekOutside = function peek(fn) { return fn.caller; };
vm.runInThisContext(fs.readFileSync(process.argv[2], "utf8"));
