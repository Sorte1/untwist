var arr = function () { return ["a", "b", "c"]; }; var h9 = 1;
{ function h9() { return 2; } let al = arr(); for (;;) { try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } } }
console.log(typeof h9);
