function outer(arr) {
  {
    var al = arr();
    while (true) {
      try { var h = parseInt(al[0]); if (h === 3) break; else al.push(al.shift()); } catch (e) { al.push(al.shift()); }
    }
  }
  return "checksum " + h;
}
console.log(outer(function () { return ["1", "2", "3"]; }));
