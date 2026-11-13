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
import('./lazy.chunk.js').then(lazy => console.log("lazy " + lazy.value));
