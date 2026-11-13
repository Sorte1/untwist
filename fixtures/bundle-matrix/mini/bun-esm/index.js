var __esm = (fn, res, err) => () => {
  if (fn)
    try {
      res = fn(fn = 0);
    } catch (e) {
      err = [e];
    }
  if (err)
    throw err[0];
  return res;
};

// lazy.js
var value = 42;
var init_lazy = __esm(() => {
  console.log("eval lazy");
});

// cjs.js
console.log("eval cjs");
var $twice = function(n) {
  return n * 2;
};

// index.js
console.log("eval index " + $twice(21));
Promise.resolve().then(() => (init_lazy(), {})).then((lazy) => console.log("lazy " + value));
