// fixtures/jsx/automatic-members.js
jsxs(React.Fragment, { children: [jsx("span", {}), "test"] });
jsx(React.Fragment, {}, o);
jsxs(React.Fragment, { children: [x, ...y] });
(0, r.jsx)("div", {});
r.jsxs(Foo, { bar: 1, children: [r.jsx(Child, {})] });
f(/*#__PURE__*/_jsx("h1", { children: /*#__PURE__*/_jsx("div", {}) }));
_jsx(Foo, { bar: 'abc' });
_jsx(Foo, { bar: 'a\\nbc' });
_jsx(Foo, { bar: 'ab\\tc' });
_jsx(Foo, { bar: 'ab"c' });
_jsx(Foo, { bar: "ab'c" });
jsx("div", { children: jsxs(React.Fragment, { children: ["x", jsx("b", {})] }) });
