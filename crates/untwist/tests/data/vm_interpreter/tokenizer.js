var text = "QUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJDQUJD";
function scan(s) {
  var i = 0, n = 0;
  while (true) {
    var c = s.charCodeAt(i++);
    if (c !== c) break;
    switch (c) {
      case 9: n += 0; break;
      case 10: n += 1; break;
      case 13: n += 2; break;
      case 32: n += 3; break;
      case 34: n += 4; break;
      case 39: n += 5; break;
      case 40: n += 6; break;
      case 41: n += 7; break;
      case 44: n += 8; break;
      case 46: n += 9; break;
      case 47: n += 10; break;
      case 58: n += 11; break;
      case 59: n += 12; break;
      case 60: n += 13; break;
      case 61: n += 14; break;
      case 62: n += 15; break;
      case 63: n += 16; break;
      case 91: n += 17; break;
      case 93: n += 18; break;
      case 123: n += 19; break;
      case 125: n += 20; break;
    }
  }
  return n;
}
console.log(scan(text + " ;"));
