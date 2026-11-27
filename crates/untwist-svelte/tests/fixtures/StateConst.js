import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button class="a">make</button> <button class="b">bump</button> <p> </p>`, 1);

export default function StateConst($$anchor) {
	const zero = 0;
	let box = $.state(zero);

	function make() {
		$.set(box, { n: 0 }, true);
	}

	function bump() {
		$.get(box).n++;
	}

	var fragment = root();
	var button = $.first_child(fragment);
	var button_1 = $.sibling(button, 2);
	var p = $.sibling(button_1, 2);
	var text = $.only_child(p, true);

	$.template_effect(() => $.set_text(text, typeof $.get(box) === 'number' ? $.get(box) : $.get(box).n));
	$.delegated('click', button, make);
	$.delegated('click', button_1, bump);
	$.append($$anchor, fragment);
}

$.delegate(['click']);