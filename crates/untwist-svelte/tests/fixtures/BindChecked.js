import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p>on</p>`);
var root_1 = $.from_html(`<label><input type="checkbox"/> toggle</label> <!>`, 1);

export default function BindChecked($$anchor) {
	let on = $.state(false);
	var fragment = root_1();
	var label = $.first_child(fragment);
	var input = $.child(label);

	$.remove_input_defaults(input);
	$.next();
	$.reset(label);

	var node = $.sibling(label, 2);

	{
		var consequent = ($$anchor) => {
			var p = root();

			$.append($$anchor, p);
		};

		$.if(node, ($$render) => {
			if ($.get(on)) $$render(consequent);
		});
	}

	$.bind_checked(input, () => $.get(on), ($$value) => $.set(on, $$value));
	$.append($$anchor, fragment);
}