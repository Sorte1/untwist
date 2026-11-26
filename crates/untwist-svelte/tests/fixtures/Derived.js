import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button> </button>`);

export default function Derived($$anchor, $$props) {
	$.push($$props, true);

	let n = $.state(1);
	let a = $.derived(() => $.get(n) * 2);

	let b = $.derived(() => {
		let total = 0;

		for (let i = 0; i < $.get(n); i++) total += i;

		return total;
	});

	let c = $.derived(() => $.get(n) + 1);
	let d = $.derived(() => (() => $.get(n) + 2)());

	let $$d = $.derived(() => ({ x: $.get(n), y: $.get(n) * 3 })),
		x = $.derived(() => $.get($$d).x),
		y = $.derived(() => $.get($$d).y);

	var button = root();
	var text = $.only_child(button);

	$.template_effect(() => $.set_text(text, `${$.get(a) ?? ''} ${$.get(b) ?? ''} ${$.get(c) ?? ''} ${$.get(d) ?? ''} ${$.get(x) ?? ''} ${$.get(y) ?? ''}`));
	$.delegated('click', button, () => $.update(n));
	$.append($$anchor, button);
	$.pop();
}

$.delegate(['click']);