import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<ul>`),
  _tmpl$2 = /*#__PURE__*/_$template(`<p> left`),
  _tmpl$3 = /*#__PURE__*/_$template(`<div class=todo><input><button class=add>add`),
  _tmpl$4 = /*#__PURE__*/_$template(`<p>nothing to do`),
  _tmpl$5 = /*#__PURE__*/_$template(`<li><button class=rm>x`);
import { createSignal, For, Show } from "solid-js";
export default function Todo() {
  const [todos, setTodos] = createSignal([]);
  const [draft, setDraft] = createSignal("");
  let next = 0;
  const add = () => {
    if (!draft()) return;
    setTodos([...todos(), {
      id: next++,
      text: draft()
    }]);
    setDraft("");
  };
  return (() => {
    var _el$ = _tmpl$3(),
      _el$2 = _el$.firstChild,
      _el$3 = _el$2.nextSibling;
    _el$2.$$input = e => setDraft(e.currentTarget.value);
    _el$3.$$click = add;
    _$insert(_el$, _$createComponent(Show, {
      get when() {
        return todos().length;
      },
      get fallback() {
        return _tmpl$4();
      },
      get children() {
        return [(() => {
          var _el$4 = _tmpl$();
          _$insert(_el$4, _$createComponent(For, {
            get each() {
              return todos();
            },
            children: todo => (() => {
              var _el$8 = _tmpl$5(),
                _el$9 = _el$8.firstChild;
              _$insert(_el$8, () => todo.text, _el$9);
              _el$9.$$click = () => setTodos(todos().filter(t => t.id !== todo.id));
              return _el$8;
            })()
          }));
          return _el$4;
        })(), (() => {
          var _el$5 = _tmpl$2(),
            _el$6 = _el$5.firstChild;
          _$insert(_el$5, () => todos().length, _el$6);
          return _el$5;
        })()];
      }
    }), null);
    _$effect(() => _el$2.value = draft());
    return _el$;
  })();
}
_$delegateEvents(["input", "click"]);