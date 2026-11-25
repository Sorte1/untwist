import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<label><input type="radio"/> </label>`);
var root_1 = $.from_html(`<p> </p> <!>`, 1);

export default function Select($$anchor) {
	const binding_group = [];
	let picked = $.state('b');
	let options = ['a', 'b', 'c'];
	var fragment = root_1();
	var p = $.first_child(fragment);
	var text = $.only_child(p);
	var node = $.sibling(p, 2);

	$.each(node, 17, () => options, $.index, ($$anchor, option) => {
		var label = root();
		var input = $.child(label);

		$.remove_input_defaults(input);

		var input_value;
		var text_1 = $.sibling(input);

		$.reset(label);

		$.template_effect(() => {
			if (input_value !== (input_value = $.get(option))) {
				input.value = (input.__value = input_value) ?? '';
			}

			$.set_text(text_1, ` ${$.get(option) ?? ''}`);
		});

		$.bind_group(
			binding_group,
			[],
			input,
			() => {
				$.get(option);

				return $.get(picked);
			},
			($$value) => $.set(picked, $$value)
		);

		$.append($$anchor, label);
	});

	$.template_effect(() => $.set_text(text, `picked ${$.get(picked) ?? ''}`));
	$.append($$anchor, fragment);
}