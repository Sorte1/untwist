(__webpack_require__.O(0, ["vendors"], () => {
  return __webpack_require__(4821);
}), __webpack_require__.O());
__webpack_require__.f.require = (chunkId, promises) => {
  if (installedChunks[chunkId]) return;
  installedChunks[chunkId] = 1;
};
(self["webpackChunkapp"] = self["webpackChunkapp"] || []).push([
  ["vendors"],
  {
    4821: (module, __unused_exports, __webpack_require__) => {
      module.exports = __webpack_require__(730);
    },
    730: (module) => {
      module.exports = "leaf";
    }
  }
]);
