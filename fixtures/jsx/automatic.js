// fixtures/jsx/automatic.js
function TodoList() {}
function lower() {}
const components = { list: { TodoList: TodoList } };
jsx("div", {});
jsx(TodoList, {});
jsx(components.list.TodoList, {});
jsx(lower, {});
jsx(lower, {});
jsx("div", { "data-hover": "tooltip", style: { display: "block" } });
jsx("div", { ...props });
jsx("div", { children: jsxs("span", { children: ["Hello ", name] }) });
jsx("div", { children: ".style { color: red; }" });
jsx("div", {}, "test");
jsx("div", { children: [1] });
