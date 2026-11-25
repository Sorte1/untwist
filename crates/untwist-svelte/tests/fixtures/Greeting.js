import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<h1> </h1>`);

export default function Greeting($$anchor, $$props) {
	let name = $.prop($$props, 'name', 3, 'world'),
		excited = $.prop($$props, 'excited', 3, false);

	var h1 = root();
	var text = $.only_child(h1);

	$.template_effect(() => $.set_text(text, `Hello ${name() ?? ''}${excited() ? '!' : '.'}`));
	$.append($$anchor, h1);
}