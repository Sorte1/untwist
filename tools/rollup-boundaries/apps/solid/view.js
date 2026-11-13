import { createMemo } from "solid-js";

export function visible(todos) {
  return createMemo(() => {
    const { filter } = todos.state;
    return todos.state.todos.filter((todo) => filter === "all" || (filter === "done") === todo.done);
  });
}

export function line(todo) {
  return (todo.done ? "[x] " : "[ ] ") + todo.title;
}
