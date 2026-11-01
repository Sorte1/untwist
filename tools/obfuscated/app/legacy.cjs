const host = eval("require;");

module.exports = {
  describe() {
    return "legacy module " + typeof module.exports;
  },
  loader() {
    return "host require is a " + typeof host;
  },
  checksum(text) {
    let hash = 7;
    for (let i = 0; i < text.length; i++) hash = (hash * 31 + text.charCodeAt(i)) % 1000003;
    return "checksum " + hash;
  },
};
