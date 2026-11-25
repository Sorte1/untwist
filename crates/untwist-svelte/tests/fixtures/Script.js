import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button>up</button> <button>down</button> <output> </output>`, 1);

export default function Script($$anchor, $$props) {
	const LIMIT = 3;
	let start = $.prop($$props, 'start', 3, 0);
	let value = $.state($.proxy(start()));
	let capped = $.derived(() => Math.min($.get(value), LIMIT));

	function step(by) {
		$.set(value, $.get(value) + by);
	}

	var fragment = root();
	var button = $.first_child(fragment);
	var button_1 = $.sibling(button, 2);
	var output = $.sibling(button_1, 2);
	var text = $.only_child(output);

	$.template_effect(() => $.set_text(text, `${$.get(capped) ?? ''} of 3`));
	$.delegated('click', button, () => step(1));
	$.delegated('click', button_1, () => step(-1));
	$.append($$anchor, fragment);
}

$.delegate(['click']);