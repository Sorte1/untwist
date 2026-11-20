function T(f){ try { return String(f()) } catch (e) { return e.name } }
c9 = "constructor"; T(function(){ go9(["de", "ad"].join("") + " = function(){ return 'HACK' }")() });
show();
setTimeout(function(){ show() }, 30);
