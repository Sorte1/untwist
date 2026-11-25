import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<div></div> <button>swap</button>`, 1);

export default function RawHtml($$anchor) {
	let markup = $.state('<b>bold</b>');
	var fragment = root();
	var div = $.first_child(fragment);

	$.html(div, () => $.get(markup), true);
	$.reset(div);

	var button = $.sibling(div, 2);

	$.delegated('click', button, () => $.set(markup, '<i>it</i>'));
	$.append($$anchor, fragment);
}

$.delegate(['click']);