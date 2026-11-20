function T(f){ try { return String(f()) } catch (e) { return e.name } }
c9 = "constructor"; T(function(){ go9(["ar", "r"].join("") + " = function(){ return 'HACK' }")() });
show();
setTimeout(function(){ show() }, 30);
