export const kind = (function (m) {
  return typeof m;
})(module);
export function loadedNow() {
  return (function (m) {
    return m.loaded;
  })(module);
}
