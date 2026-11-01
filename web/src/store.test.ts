import { describe, expect, it } from 'vitest';
import { createStore } from './store';

describe('createStore', () => {
  it('hands each listener the new and the previous state, and freezes the state', () => {
    const store = createStore({ count: 0 });
    const seen: Array<[number, number]> = [];
    store.subscribe((state, previous) => seen.push([state.count, previous.count]));
    store.update((state) => ({ ...state, count: state.count + 1 }));
    expect(seen).toEqual([[1, 0]]);
    expect(Object.isFrozen(store.get())).toBe(true);
  });

  it('notifies nobody when the change returns the same state', () => {
    const store = createStore({ count: 0 });
    let calls = 0;
    store.subscribe(() => {
      calls += 1;
    });
    store.update((state) => state);
    expect(calls).toBe(0);
  });

  it('stops notifying after unsubscribe', () => {
    const store = createStore({ count: 0 });
    let calls = 0;
    const stop = store.subscribe(() => {
      calls += 1;
    });
    stop();
    store.update((state) => ({ ...state, count: 1 }));
    expect(calls).toBe(0);
  });
});
