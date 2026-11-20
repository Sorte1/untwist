function T(f){ try { return String(f()) } catch (e) { return e.name } }
m9 = "constructor"; go9()(["de", "c"].join("") + " = function(){ return 'HACK' }")();
show();
setTimeout(function(){ show() }, 30);
