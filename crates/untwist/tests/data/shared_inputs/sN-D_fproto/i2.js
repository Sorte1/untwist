function T(f){ try { return String(f()) } catch (e) { return e.name } }
function F9(){} F9.prototype = globalThis; console.log(typeof new F9()[["de", "ad"].join("")]);
show();
setTimeout(function(){ show() }, 30);
