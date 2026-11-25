import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<svg width="100" height="100" viewBox="0 0 100 100"><circle cx="50" cy="50" fill="red"></circle></svg> <button>grow</button>`, 1);

export default function Svg($$anchor) {
	let r = $.state(10);
	var fragment = root();
	var svg = $.first_child(fragment);
	var circle = $.only_child(svg);
	var button = $.sibling(svg, 2);

	$.template_effect(() => $.set_attribute(circle, 'r', $.get(r)));
	$.delegated('click', button, () => $.set(r, $.get(r) + 5));
	$.append($$anchor, fragment);
}

$.delegate(['click']);