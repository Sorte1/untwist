import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p> </p>`);
var root_1 = $.from_html(`<p>loading</p>`);

export default function Await($$anchor, $$props) {
	$.push($$props, true);

	let value = $.prop($$props, 'value', 3, 'done'),
		fail = $.prop($$props, 'fail', 3, false);

	let promise = $.proxy(new Promise((resolve, reject) => setTimeout(() => fail() ? reject(new Error('bad')) : resolve(value()), 5)));
	var fragment = $.comment();
	var node = $.first_child(fragment);

	$.await(
		node,
		() => promise,
		($$anchor) => {
			var p_2 = root_1();

			$.append($$anchor, p_2);
		},
		($$anchor, result) => {
			var p = root();
			var text = $.only_child(p);

			$.template_effect(() => $.set_text(text, `got ${$.get(result) ?? ''}`));
			$.append($$anchor, p);
		},
		($$anchor, error) => {
			var p_1 = root();
			var text_1 = $.only_child(p_1);

			$.template_effect(() => $.set_text(text_1, `error ${$.get(error).message ?? ''}`));
			$.append($$anchor, p_1);
		}
	);

	$.append($$anchor, fragment);
	$.pop();
}