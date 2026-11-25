import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div> </div> <button>flip</button>`, 1);

export default function Directives($$anchor) {
	let active = $.state(false);
	let color = $.state('blue');
	var fragment = root();
	var div = $.first_child(fragment);
	let classes;
	let styles;
	var text = $.only_child(div, true);
	var button = $.sibling(div, 2);

	$.template_effect(() => {
		classes = $.set_class(div, 1, '', null, classes, { active: $.get(active), big: !$.get(active) });
		styles = $.set_style(div, '', styles, { color: $.get(color) });
		$.set_text(text, $.get(active) ? 'yes' : 'no');
	});

	$.delegated('click', button, () => {
		$.set(active, !$.get(active));
		$.set(color, 'green');
	});

	$.append($$anchor, fragment);
}

$.delegate(['click']);