function length(a) { return a + 1; }
[].push.apply(this, []);
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
