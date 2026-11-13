class Config {
  static defaults = { mode: "fast" };
  static ready;
  static {
    this.ready = Object.keys(this.defaults).length > 0;
    Config.loaded = "yes";
  }
  static {
    let local = 3;
    this.triple = local * 3;
  }
}
console.log(Config.ready, Config.loaded, Config.triple, typeof local);
