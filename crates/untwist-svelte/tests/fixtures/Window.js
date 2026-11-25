import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p> </p>`);

export default function Window($$anchor) {
	let key = $.state('');
	var p = root();

	$.event('keydown', $.window, (event) => $.set(key, event.key, true));

	var text = $.only_child(p, true);

	$.template_effect(() => $.set_text(text, $.get(key)));
	$.append($$anchor, p);
}