const { readFileSync } = require("node:fs");
const vm = require("node:vm");

const file = process.argv[2];
vm.runInThisContext(readFileSync(file, "utf8"), { filename: file });
