import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';
import Badge from './Badge.svelte';

var root = $.from_html(`<!> <!> <p> </p> <button class="more">more</button>`, 1);

export default function Parent($$anchor) {
	let picked = $.state('none');
	let count = $.state(1);
	var fragment = root();
	var node = $.first_child(fragment);

	Badge(node, {
		label: 'first',
		onpick: (value) => $.set(picked, value, true),
		children: ($$anchor, $$slotProps) => {
			$.next();

			var text = $.text();

			$.template_effect(() => $.set_text(text, `inner ${$.get(count) ?? ''}`));
			$.append($$anchor, text);
		},
		$$slots: { default: true }
	});

	var node_1 = $.sibling(node, 2);

	Badge(node_1, { label: 'second', tone: 'warn' });

	var p = $.sibling(node_1, 2);
	var text_1 = $.only_child(p);
	var button = $.sibling(p, 2);

	$.template_effect(() => $.set_text(text_1, `picked ${$.get(picked) ?? ''}`));
	$.delegated('click', button, () => $.update(count));
	$.append($$anchor, fragment);
}

$.delegate(['click']);