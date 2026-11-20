function outer(arr) { { using r = { [Symbol.dispose]: function () { console.log("disposed"); } }; var al = arr(); for (;;) { try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } } } console.log("after"); return "done"; }
console.log(outer(function () { return ["a", "b", "c"]; }));
