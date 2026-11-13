import { createStore, produce } from "solid-js/store";
import { batch } from "solid-js";

export function createTodos() {
  const [state, setState] = createStore({ todos: [], filter: "all" });
  let next = 1;
  return {
    state,
    add(title) {
      setState(produce((draft) => {
        draft.todos.push({ id: next++, title, done: false });
      }));
    },
    toggle(id) {
      setState("todos", (todo) => todo.id === id, "done", (done) => !done);
    },
    filter(name) {
      batch(() => setState("filter", name));
    },
  };
}
