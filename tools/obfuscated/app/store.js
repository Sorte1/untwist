export function createStore(initial) {
  const state = { ...initial };
  const log = [];
  return {
    get(key) {
      return Object.prototype.hasOwnProperty.call(state, key) ? state[key] : undefined;
    },
    update(key, value) {
      log.push(key + ":" + state[key] + "->" + value);
      state[key] = value;
    },
    snapshot() {
      return { ...state };
    },
    history() {
      return log.slice();
    },
  };
}
