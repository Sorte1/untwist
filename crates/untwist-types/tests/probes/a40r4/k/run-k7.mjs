Object.prototype.__defineGetter__("exports", function () { return this; });
const P = exports.Promise;
P.prototype.then = function () { return 5; };
async function h() { return 1; }
const v = await h();
const w = await Promise.resolve("s");
const t = h().then((x) => x);
console.log(typeof v, typeof w, typeof t);
export {};
