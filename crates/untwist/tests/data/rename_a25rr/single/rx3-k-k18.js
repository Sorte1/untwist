var q = 1; function zz() { return 2; }
console.log(q + zz(), new Function("try { return a; } catch (e) { return 'none'; }")());
