import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button class="replace"> </button> <button class="push"> </button>`, 1);

export default function RawState($$anchor) {
	let items = $.state([1, 2]);
	let pushed = [1];
	var fragment = root();
	var button = $.first_child(fragment);
	var text = $.only_child(button, true);
	var button_1 = $.sibling(button, 2);
	var text_1 = $.only_child(button_1, true);

	$.template_effect(() => {
		$.set_text(text, $.get(items).length);
		$.set_text(text_1, pushed.length);
	});

	$.delegated('click', button, () => $.set(items, [...$.get(items), $.get(items).length + 1]));
	$.delegated('click', button_1, () => pushed.push(2));
	$.append($$anchor, fragment);
}

$.delegate(['click']);