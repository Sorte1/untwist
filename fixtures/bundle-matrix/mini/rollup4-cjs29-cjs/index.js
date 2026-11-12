'use strict';

var cjs = {};

var hasRequiredCjs;

function requireCjs () {
	if (hasRequiredCjs) return cjs;
	hasRequiredCjs = 1;
	console.log("eval cjs");
	cjs.twice = function (n) {
	  return n * 2;
	};
	return cjs;
}

var cjsExports = requireCjs();

console.log("eval index " + cjsExports.twice(21));
Promise.resolve().then(function () { return lazy; }).then(lazy => console.log("lazy " + lazy.value));

console.log("eval lazy");
const value = 42;

var lazy = /*#__PURE__*/Object.freeze({
  __proto__: null,
  value: value
});
