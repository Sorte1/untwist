async function outer(arr) { { await using r = { [Symbol.asyncDispose]: async function () { console.log("disposed"); } }; var al = arr(); for (;;) { try { if (al[0] === "c") break; al.push(al.shift()); } catch (e) { al.push(al.shift()); } } } return "done"; }
outer(function () { return ["a", "b", "c"]; }).then(console.log);
