console.log("host evaluated", typeof hostConfig);
export function configured() {
  return typeof hostConfig !== "undefined" && hostConfig !== null ? JSON.stringify(hostConfig) : "none";
}
export function hooked(n) {
  return typeof hostHook === "function" ? hostHook(n) : "none";
}
