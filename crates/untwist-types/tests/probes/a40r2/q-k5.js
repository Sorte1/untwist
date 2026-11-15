Reflect.apply(console.log.constructor, null, ["Math.random = () => 's'"])();
(function () { const r = Math.random(); const p = parseInt("12"); console.log(typeof r, typeof p); })();
