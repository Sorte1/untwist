import { template as _$template } from "solid-js/web";
import { delegateEvents as _$delegateEvents } from "solid-js/web";
import { effect as _$effect } from "solid-js/web";
import { createComponent as _$createComponent } from "solid-js/web";
import { insert as _$insert } from "solid-js/web";
var _tmpl$ = /*#__PURE__*/_$template(`<button>more`),
  _tmpl$2 = /*#__PURE__*/_$template(`<div class=mixed><h3>`),
  _tmpl$3 = /*#__PURE__*/_$template(`<span class=tag>`),
  _tmpl$4 = /*#__PURE__*/_$template(`<s>hidden`);
import { createSignal, Show, For } from "solid-js";
export default function Mixed(props) {
  const [open, setOpen] = createSignal(true);
  const [tags, setTags] = createSignal(["x"]);
  return (() => {
    var _el$ = _tmpl$2(),
      _el$2 = _el$.firstChild;
    _el$2.$$click = () => setOpen(!open());
    _$insert(_el$2, () => props.heading);
    _$insert(_el$, _$createComponent(Show, {
      get when() {
        return open();
      },
      get children() {
        return [_$createComponent(For, {
          get each() {
            return tags();
          },
          children: tag => _$createComponent(Show, {
            when: tag !== "hide",
            get fallback() {
              return _tmpl$4();
            },
            get children() {
              var _el$4 = _tmpl$3();
              _$insert(_el$4, tag);
              return _el$4;
            }
          })
        }), (() => {
          var _el$3 = _tmpl$();
          _el$3.$$click = () => setTags([...tags(), "hide", "y"]);
          return _el$3;
        })()];
      }
    }), null);
    _$effect(() => _el$.classList.toggle("open", !!open()));
    return _el$;
  })();
}
_$delegateEvents(["click"]);