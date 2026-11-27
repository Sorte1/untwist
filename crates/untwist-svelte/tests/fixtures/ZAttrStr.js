import 'svelte/internal/disclose-version';
import 'svelte/internal/flags/legacy';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p>t</p> <input/> <a>l</a>`, 1);

export default function ZAttrStr($$anchor) {
	const a = 'x';
	const q = '"';
	const amp = '&amp;';
	const br = '{';
	const nl = '\r\n\t';
	const bt = '`${';
	const lt = '<b>';
	var fragment = root();
	var p = $.first_child(fragment);

	$.set_attribute(p, 'title', '1 x " &amp; {y} \r\n	| `${ <b> < {');
	$.set_attribute(p, 'data-e', 'x');
	$.set_attribute(p, 'alt', '""');
	$.set_class(p, 1, 'c x  d');
	$.set_style(p, 'color: red; x');

	var input = $.sibling(p, 2);

	$.remove_input_defaults(input);
	$.set_value(input, 'x "&amp;');
	$.set_attribute(input, 'placeholder', ' x ');

	var a_1 = $.sibling(input, 2);

	$.set_attribute(a_1, 'href', '?a=1&b=x&c');
	$.append($$anchor, fragment);
}