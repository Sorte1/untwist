// fixtures/jsx/classic.js
function TodoList() {}
function lower() {}
const components = { list: { TodoList: TodoList } };
React.createElement("div", null);
React.createElement(TodoList, null);
React.createElement(components.list.TodoList, null);
React.createElement(lower, null);
React.createElement(lower, null);
React.createElement("div", { "data-hover": "tooltip", style: { display: "block" } });
React.createElement("div", { ...props });
React.createElement("div", null, React.createElement("span", null, "Hello ", name));
React.createElement("div", null, ...children);
React.createElement("div", null, ".style { color: red; }");
React.createElement(Foo, { bar: 'abc' });
React.createElement(Foo, { bar: 'a\\nbc' });
React.createElement(Foo, { bar: 'ab\\tc' });
React.createElement(Foo, { bar: 'ab"c' });
React.createElement(Foo, { bar: "ab'c" });
