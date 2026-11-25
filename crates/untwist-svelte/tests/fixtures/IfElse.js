import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p>zero</p>`);
var root_1 = $.from_html(`<p>one</p>`);
var root_2 = $.from_html(`<p> </p>`);
var root_3 = $.from_html(`<button>next</button> <!>`, 1);

export default function IfElse($$anchor) {
	let n = $.state(0);
	var fragment = root_3();
	var button = $.first_child(fragment);
	var node = $.sibling(button, 2);

	{
		var consequent = ($$anchor) => {
			var p = root();

			$.append($$anchor, p);
		};

		var consequent_1 = ($$anchor) => {
			var p_1 = root_1();

			$.append($$anchor, p_1);
		};

		var alternate = ($$anchor) => {
			var p_2 = root_2();
			var text = $.only_child(p_2);

			$.template_effect(() => $.set_text(text, `many: ${$.get(n) ?? ''}`));
			$.append($$anchor, p_2);
		};

		$.if(node, ($$render) => {
			if ($.get(n) === 0) $$render(consequent); else if ($.get(n) === 1) $$render(consequent_1, 1); else $$render(alternate, -1);
		});
	}

	$.delegated('click', button, () => $.update(n));
	$.append($$anchor, fragment);
}

$.delegate(['click']);