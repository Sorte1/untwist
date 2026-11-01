export type Listener<S> = (state: S, previous: S) => void;

export interface Store<S> {
  get(): S;
  update(change: (state: S) => S): void;
  subscribe(listener: Listener<S>): () => void;
}

export function createStore<S extends object>(initial: S): Store<S> {
  let state: S = Object.freeze({ ...initial });
  const listeners = new Set<Listener<S>>();
  return {
    get: () => state,
    update(change) {
      const previous = state;
      const next = change(state);
      if (next === previous) {
        return;
      }
      state = Object.freeze(next);
      for (const listener of listeners) {
        listener(state, previous);
      }
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
