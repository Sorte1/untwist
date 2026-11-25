import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<input/> <button>go</button> <p> </p>`, 1);

export default function Events($$anchor) {
	let log = $.state('');

	function onInput(event) {
		$.set(log, 'input ' + event.target.value);
	}

	var fragment = root();
	var input = $.first_child(fragment);
	var button = $.sibling(input, 2);
	var p = $.sibling(button, 2);
	var text = $.only_child(p, true);

	$.template_effect(() => $.set_text(text, $.get(log)));
	$.delegated('input', input, onInput);
	$.delegated('click', button, () => $.set(log, 'clicked'));
	$.event('mouseenter', button, () => $.set(log, 'hover'));
	$.append($$anchor, fragment);
}

$.delegate(['input', 'click']);