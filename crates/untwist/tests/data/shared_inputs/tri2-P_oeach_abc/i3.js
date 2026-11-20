function T(f){ try { return String(f()) } catch (e) { return e.name } }
o9 = this; console.log(names9());
show();
setTimeout(function(){ show() }, 30);
