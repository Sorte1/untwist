import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';
import Badge from './Badge.svelte';

var root = $.from_html(`<!> <button class="swap">swap</button>`, 1);

export default function SpreadChild($$anchor) {
	let props = $.state($.proxy({ label: 'spread', tone: 'warn' }));
	var fragment = root();
	var node = $.first_child(fragment);

	Badge(node, $.spread_props(() => $.get(props)));

	var button = $.sibling(node, 2);

	$.delegated('click', button, () => $.set(props, { label: 'other' }, true));
	$.append($$anchor, fragment);
}

$.delegate(['click']);