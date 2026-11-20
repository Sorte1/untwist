var arr = ["hello ", "bye "]; function dec(i) { return arr[i]; } function show(){ var out = dec(0) + "abc" + dec(1); console.log(out); }
function T(f){ try { return String(f()) } catch (e) { return e.name } }
m9 = "constructor"; go9()(["de", "c"].join("") + " = function(){ return 'HACK' }")();
show();
setTimeout(function(){ show() }, 30);
