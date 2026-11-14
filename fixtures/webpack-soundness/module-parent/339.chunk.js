exports.ids = [339];
exports.modules = {
339(module, exports) {
console.log("m339", typeof module.parent, typeof module.filename, module.loaded, typeof module.id, Object.keys(module).join());
if (!module.parent) { console.log("running as main"); }
}
};
