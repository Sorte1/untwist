function outer(arr) { var h = 1; { let al = arr(); function h() { return 2; } for (;;) { try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } } } return typeof h; }
console.log(outer(function () { return ["a", "b", "c"]; }));
