// @virtualize
function checksum(text, seed) {
  let hash = seed;
  for (let i = 0; i < text.length; i++) {
    hash = (hash * 31 + text.charCodeAt(i)) % 1000003;
  }
  return hash;
}

function plain(list) {
  return list.map((value) => value * 2).join(",");
}

console.log("checksum", checksum("untwist", 7), checksum("vm", 1));
console.log("plain", plain([1, 2, 3]));
