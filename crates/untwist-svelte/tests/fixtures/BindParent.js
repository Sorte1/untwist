import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';
import Field from './Field.svelte';

var root = $.from_html(`<!> <p> </p>`, 1);

export default function BindParent($$anchor) {
	let n = $.state(1);
	var fragment = root();
	var node = $.first_child(fragment);

	Field(node, {
		get value() {
			return $.get(n);
		},

		set value($$value) {
			$.set(n, $$value, true);
		}
	});

	var p = $.sibling(node, 2);
	var text = $.only_child(p);

	$.template_effect(() => $.set_text(text, `n is ${$.get(n) ?? ''}`));
	$.append($$anchor, fragment);
}