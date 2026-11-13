import { createRoot, createSignal, createEffect, on, untrack } from "solid-js";
import { createTodos } from "./store.js";
import { visible, line } from "./view.js";

createRoot((dispose) => {
  const todos = createTodos();
  const shown = visible(todos);
  const [tick, setTick] = createSignal(0);
  createEffect(on(shown, (list) => {
    console.log("render", untrack(tick), list.map(line).join(" / "));
  }));
  todos.add("write tests");
  todos.add("ship");
  setTick(1);
  todos.toggle(1);
  todos.filter("done");
  setTick(2);
  todos.filter("all");
  todos.add("celebrate");
  dispose();
  console.log("disposed");
});
