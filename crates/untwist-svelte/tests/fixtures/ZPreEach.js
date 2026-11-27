import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<pre>  <!></pre> <p>after</p>`, 1);

export default function ZPreEach($$anchor) {
	let items = $.proxy([1, 2]);
	var fragment = root();
	var pre = $.first_child(fragment);
	var node = $.sibling($.child(pre));

	$.each(node, 17, () => items, $.index, ($$anchor, i) => {
		$.next();

		var text = $.text();

		$.template_effect(() => $.set_text(text, `${$.get(i) ?? ''}
`));

		$.append($$anchor, text);
	});

	$.reset(pre);
	$.next(2);
	$.append($$anchor, fragment);
}