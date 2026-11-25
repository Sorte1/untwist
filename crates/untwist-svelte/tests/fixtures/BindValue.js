import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<input/> <p> </p>`, 1);

export default function BindValue($$anchor) {
	let text = $.state('start');
	var fragment = root();
	var input = $.first_child(fragment);

	$.remove_input_defaults(input);

	var p = $.sibling(input, 2);
	var text_1 = $.only_child(p, true);

	$.template_effect(($0) => $.set_text(text_1, $0), [() => $.get(text).toUpperCase()]);
	$.bind_value(input, () => $.get(text), ($$value) => $.set(text, $$value));
	$.append($$anchor, fragment);
}