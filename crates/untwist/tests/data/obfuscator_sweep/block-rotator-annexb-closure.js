function outer(arr) { var h = 1; var g = function () { return typeof h; }; { function h() { return 2; } var al = arr(); for (;;) { try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } } } return g(); }
console.log(outer(function () { return ["a", "b", "c"]; }));
