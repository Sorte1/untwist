function nested(flag) {
  const seen = [];
  if (true) {
    if (false) seen.push("dead1");
    else {
      if (1) {
        seen.push("inner");
        if (0) seen.push("dead2"); else seen.push("deep");
      }
    }
  } else {
    if (true) seen.push("dead3");
  }
  if (flag) {
    if (false) seen.push("dead4");
    seen.push("flagged");
  }
  return seen;
}
console.log(nested(true).join(","), nested(false).join(","));
