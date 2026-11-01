export default class Emitter {
  constructor() {
    this.handlers = new Map();
  }

  on(name, handler) {
    const list = this.handlers.get(name) || [];
    list.push(handler);
    this.handlers.set(name, list);
    return this;
  }

  emit(name, ...args) {
    for (const handler of this.handlers.get(name) || []) handler(...args);
    return this.handlers.has(name);
  }
}
