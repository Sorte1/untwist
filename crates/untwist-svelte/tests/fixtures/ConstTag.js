import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<i> </i>`);

export default function ConstTag($$anchor) {
	let values = $.proxy([1, 2]);
	var fragment = $.comment();
	var node = $.first_child(fragment);

	$.each(node, 17, () => values, $.index, ($$anchor, value) => {
		const double = $.derived(() => $.get(value) * 2);
		var i = root();
		var text = $.only_child(i, true);

		$.template_effect(() => $.set_text(text, $.get(double)));
		$.append($$anchor, i);
	});

	$.append($$anchor, fragment);
}