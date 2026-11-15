function w() { return w.caller === null ? "null" : w.caller.name; } function main() { var r; eval("r = w();"); return r; } console.log(main()); (function boot() { eval("console.log(w())"); })();
