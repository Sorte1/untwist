globalThis.document = {
  baseURI: "file:///app/",
  get currentScript() {
    console.log("currentScript read");
    return null;
  },
};
