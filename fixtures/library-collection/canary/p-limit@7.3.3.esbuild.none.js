(() => {
  var __defProp = Object.defineProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };

  // node_modules/p-limit/index.js
  var p_limit_exports = {};
  __export(p_limit_exports, {
    default: () => pLimit,
    limitFunction: () => limitFunction
  });

  // node_modules/yocto-queue/index.js
  var Node = class {
    value;
    next;
    constructor(value) {
      this.value = value;
    }
  };
  var Queue = class {
    #head;
    #tail;
    #size;
    constructor() {
      this.clear();
    }
    enqueue(value) {
      const node = new Node(value);
      if (this.#head) {
        this.#tail.next = node;
        this.#tail = node;
      } else {
        this.#head = node;
        this.#tail = node;
      }
      this.#size++;
    }
    dequeue() {
      const current = this.#head;
      if (!current) {
        return;
      }
      this.#head = this.#head.next;
      this.#size--;
      if (!this.#head) {
        this.#tail = void 0;
      }
      return current.value;
    }
    peek() {
      if (!this.#head) {
        return;
      }
      return this.#head.value;
    }
    clear() {
      this.#head = void 0;
      this.#tail = void 0;
      this.#size = 0;
    }
    get size() {
      return this.#size;
    }
    *[Symbol.iterator]() {
      let current = this.#head;
      while (current) {
        yield current.value;
        current = current.next;
      }
    }
    *drain() {
      while (this.#head) {
        yield this.dequeue();
      }
    }
  };

  // node_modules/p-limit/index.js
  function pLimit(concurrency) {
    let rejectOnClear = false;
    if (typeof concurrency === "object") {
      ({ concurrency, rejectOnClear = false } = concurrency);
    }
    validateConcurrency(concurrency);
    if (typeof rejectOnClear !== "boolean") {
      throw new TypeError("Expected `rejectOnClear` to be a boolean");
    }
    const queue = new Queue();
    let activeCount = 0;
    const resumeNext = () => {
      if (activeCount < concurrency && queue.size > 0) {
        activeCount++;
        queue.dequeue().run();
      }
    };
    const next = () => {
      activeCount--;
      resumeNext();
    };
    const run = async (function_, resolve, arguments_) => {
      const result = (async () => function_(...arguments_))();
      resolve(result);
      try {
        await result;
      } catch {
      }
      next();
    };
    const enqueue = (function_, resolve, reject, arguments_) => {
      const queueItem = { reject };
      new Promise((internalResolve) => {
        queueItem.run = internalResolve;
        queue.enqueue(queueItem);
      }).then(run.bind(void 0, function_, resolve, arguments_));
      if (activeCount < concurrency) {
        resumeNext();
      }
    };
    const generator = (function_, ...arguments_) => new Promise((resolve, reject) => {
      enqueue(function_, resolve, reject, arguments_);
    });
    Object.defineProperties(generator, {
      activeCount: {
        get: () => activeCount
      },
      pendingCount: {
        get: () => queue.size
      },
      clearQueue: {
        value() {
          if (!rejectOnClear) {
            queue.clear();
            return;
          }
          const abortError = AbortSignal.abort().reason;
          while (queue.size > 0) {
            queue.dequeue().reject(abortError);
          }
        }
      },
      concurrency: {
        get: () => concurrency,
        set(newConcurrency) {
          validateConcurrency(newConcurrency);
          concurrency = newConcurrency;
          queueMicrotask(() => {
            while (activeCount < concurrency && queue.size > 0) {
              resumeNext();
            }
          });
        }
      },
      map: {
        async value(iterable, function_) {
          const promises = [];
          try {
            Array.from(iterable, (value, index) => {
              const promise = generator(function_, value, index);
              promises.push(promise);
              return promise;
            });
          } catch (error) {
            for (const promise of promises) {
              promise.catch(() => {
              });
            }
            throw error;
          }
          return Promise.all(promises);
        }
      }
    });
    return generator;
  }
  function limitFunction(function_, options) {
    const limit = pLimit(options);
    const limitedFunction = (...arguments_) => limit(() => function_(...arguments_));
    Object.defineProperty(limitedFunction, "clearQueue", {
      value: limit.clearQueue
    });
    return limitedFunction;
  }
  function validateConcurrency(concurrency) {
    if (!((Number.isInteger(concurrency) || concurrency === Number.POSITIVE_INFINITY) && concurrency > 0)) {
      throw new TypeError("Expected `concurrency` to be a number from 1 and up");
    }
  }

  // .untwist-entry/esbuild.entry.mjs
  globalThis.__lib = [p_limit_exports];
})();
