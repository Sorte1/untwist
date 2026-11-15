console.log("a");
try { eval("if (0) { /(/; } console.log('in');"); } catch (e) { console.log("caught", e.name); }
console.log("b");
