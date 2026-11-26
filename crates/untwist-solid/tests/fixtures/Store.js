import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<div><button>add`),
  _tmpl$2 = /*#__PURE__*/_$template(`<label><input type=checkbox>`);
import { For } from "solid-js";
import { createStore, produce } from "solid-js/store";
export default function Store() {
  const [state, setState] = createStore({
    todos: [{
      text: "one",
      done: false
    }],
    filter: "all"
  });
  const toggle = i => setState("todos", i, "done", d => !d);
  return (() => {
    var _el$ = _tmpl$(),
      _el$2 = _el$.firstChild;
    _$insert(_el$, _$createComponent(For, {
      get each() {
        return state.todos;
      },
      children: (todo, i) => (() => {
        var _el$3 = _tmpl$2(),
          _el$4 = _el$3.firstChild;
        _el$4.addEventListener("change", () => toggle(i()));
        _$insert(_el$3, () => todo.text, null);
        _$effect(() => _el$3.classList.toggle("done", !!todo.done));
        _$effect(() => _el$4.checked = todo.done);
        return _el$3;
      })()
    }), _el$2);
    _el$2.$$click = () => setState(produce(s => s.todos.push({
      text: "two",
      done: true
    })));
    return _el$;
  })();
}
_$delegateEvents(["click"]);