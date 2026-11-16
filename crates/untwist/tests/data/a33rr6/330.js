(function () { function w() { return w.caller === null ? "null" : w.caller.name; } function main() { return eval("w()"); } console.log(main()); })();
