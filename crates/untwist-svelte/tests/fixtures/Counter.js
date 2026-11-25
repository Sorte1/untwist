import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button> </button> <p> </p>`, 1);

export default function Counter($$anchor) {
	let count = $.state(0);
	let doubled = $.derived(() => $.get(count) * 2);

	function increment() {
		$.set(count, $.get(count) + 1);
	}

	var fragment = root();
	var button = $.first_child(fragment);
	var text = $.only_child(button);
	var p = $.sibling(button, 2);
	var text_1 = $.only_child(p);

	$.template_effect(() => {
		$.set_text(text, `clicks: ${$.get(count) ?? ''}`);
		$.set_text(text_1, `doubled is ${$.get(doubled) ?? ''}`);
	});

	$.delegated('click', button, increment);
	$.append($$anchor, fragment);
}

$.delegate(['click']);