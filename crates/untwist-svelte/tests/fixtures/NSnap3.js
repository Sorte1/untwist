import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

export function snapshot(value) {
	return Object.keys(value).length;
}

var root = $.from_html(`<button> </button>`);

export default function NSnap3($$anchor, $$props) {
	$.push($$props, true);

	let form = $.proxy({ a: 1, b: 2 });
	let saved = $.state(0);

	function save() {
		$.set(saved, snapshot(form), true);
	}

	var button = root();
	var text = $.only_child(button, true);

	$.template_effect(() => $.set_text(text, $.get(saved)));
	$.delegated('click', button, save);
	$.append($$anchor, button);
	$.pop();
}

$.delegate(['click']);