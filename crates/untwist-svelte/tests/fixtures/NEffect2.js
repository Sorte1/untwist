import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

function user_effect(f) {
	console.log('mine', f());
}

function user_pre_effect(f) {
	console.log('mine-pre', f());
}

var root = $.from_html(`<button> </button>`);

export default function NEffect2($$anchor) {
	let n = $.state(1);

	user_effect(() => $.get(n));
	user_pre_effect(() => $.get(n) + 1);

	var button = root();
	var text = $.only_child(button, true);

	$.template_effect(() => $.set_text(text, $.get(n)));
	$.delegated('click', button, () => $.update(n));
	$.append($$anchor, button);
}

$.delegate(['click']);