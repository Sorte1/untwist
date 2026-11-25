import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button> </button>`);

export default function EffectPre($$anchor, $$props) {
	$.push($$props, true);

	let count = $.state(0);

	$.user_pre_effect(() => {
		console.log('before', $.get(count));
	});

	$.user_effect(() => {
		console.log('after', $.get(count));

		return () => console.log('cleanup', $.get(count));
	});

	var button = root();
	var text = $.only_child(button, true);

	$.template_effect(() => $.set_text(text, $.get(count)));
	$.delegated('click', button, () => $.set(count, $.get(count) + 2));
	$.append($$anchor, button);
	$.pop();
}

$.delegate(['click']);