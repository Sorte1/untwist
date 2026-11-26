import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div>spread</div> <button>bump</button>`, 1);

export default function SpreadAttrs($$anchor) {
	let extra = $.state($.proxy({ title: 'hello', 'data-n': 1 }));
	var fragment = root();
	var div = $.first_child(fragment);

	$.attribute_effect(div, () => ({ ...$.get(extra), class: 'box' }));

	var button = $.sibling(div, 2);

	$.delegated('click', button, () => $.set(extra, { ...$.get(extra), 'data-n': $.get(extra)['data-n'] + 1 }, true));
	$.append($$anchor, fragment);
}

$.delegate(['click']);