import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<!> <button>swap</button>`, 1);

export default function DynamicElement($$anchor) {
	let tag = $.state('h1');
	let attrs = $.proxy({ title: 'x' });
	var fragment = root();
	var node = $.first_child(fragment);

	$.element(node, () => $.get(tag), false, ($$element, $$anchor) => {
		$.attribute_effect($$element, () => ({ class: 'dyn', ...attrs }));

		var text = $.text('heading');

		$.append($$anchor, text);
	});

	var button = $.sibling(node, 2);

	$.delegated('click', button, () => $.set(tag, $.get(tag) === 'h1' ? 'h2' : 'h1', true));
	$.append($$anchor, fragment);
}

$.delegate(['click']);