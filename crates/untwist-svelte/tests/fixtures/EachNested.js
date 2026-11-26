import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p> </p>`);
var root_1 = $.from_html(`<span> </span>`);
var root_2 = $.from_html(`<!> <!> <button>flip</button>`, 1);

export default function EachNested($$anchor) {
	let rows = $.proxy([{ id: 1, pos: { x: 3 } }, { id: 2, pos: {} }]);
	var fragment = root_2();
	var node = $.first_child(fragment);

	$.each(node, 19, () => rows, ({ id, pos: { x = 9 } }) => id, ($$anchor, $$item, i) => {
		let id = () => $.get($$item).id;
		let x = $.derived_safe_equal(() => $.fallback($.get($$item).pos.x, 9));
		var p = root();
		var text = $.only_child(p);

		$.template_effect(() => $.set_text(text, `${$.get(i) ?? ''}: ${id() ?? ''} at ${$.get(x) ?? ''}`));
		$.append($$anchor, p);
	});

	var node_1 = $.sibling(node, 2);

	$.each(node_1, 16, () => [[1, 2], [3]], $.index, ($$anchor, $$item) => {
		var $$array = $.derived(() => $.to_array($$item, 2));
		let a = () => $.get($$array)[0];
		let b = $.derived_safe_equal(() => $.fallback($.get($$array)[1], 0));
		var span = root_1();
		var text_1 = $.only_child(span);

		$.template_effect(() => $.set_text(text_1, `${a() ?? ''}-${$.get(b) ?? ''}`));
		$.append($$anchor, span);
	});

	var button = $.sibling(node_1, 2);

	$.delegated('click', button, () => rows.reverse());
	$.append($$anchor, fragment);
}

$.delegate(['click']);