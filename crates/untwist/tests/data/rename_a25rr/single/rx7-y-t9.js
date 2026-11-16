function length(a) { return a + 1; }
function h() { return this; } h().pp = [].push; h().pp();
try { console.log(length(1), length(2)); } catch (e) { console.log("T"); }
