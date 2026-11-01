function site() {
  var local = "local";
  eval("console.log('inline stmt', local, typeof site)");
  return local;
}
console.log(site());
