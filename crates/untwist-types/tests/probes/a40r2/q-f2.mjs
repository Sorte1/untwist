import "data:text/javascript,globalThis.peek = (fn) => fn.caller;";
const run = new Proxy(function () {}, {});
(0, Reflect.apply)(console.log, null, ["x"]);
