function run(code) {
  var ip = 0, acc = 0;
  while (true) {
    var op = code[ip++];
    if (op === undefined) break;
    switch (op) {
      case 0: acc = acc * 3 + 0; break;
      case 1: acc = acc * 3 + 1; break;
      case 2: acc = acc * 3 + 2; break;
      case 3: acc = acc * 3 + 3; break;
      case 4: acc = acc * 3 + 4; break;
      case 5: acc = acc * 3 + 5; break;
      case 6: acc = acc * 3 + 6; break;
      case 7: acc = acc * 3 + 7; break;
      case 8: acc = acc * 3 + 8; break;
      case 9: acc = acc * 3 + 9; break;
      case 10: acc = acc * 3 + 10; break;
      case 11: acc = acc * 3 + 11; break;
      case 12: acc = acc * 3 + 12; break;
      case 13: acc = acc * 3 + 13; break;
      case 14: acc = acc * 3 + 14; break;
      case 15: acc = acc * 3 + 15; break;
      case 16: acc = acc * 3 + 16; break;
      case 17: acc = acc * 3 + 17; break;
      case 18: acc = acc * 3 + 18; break;
      case 19: acc = acc * 3 + 19; break;
    }
  }
  return acc;
}
console.log(run([1, 2, 3]));
