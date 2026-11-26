import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p class="err"> </p>`);
var root_1 = $.from_html(`<p> </p>`);
var root_2 = $.from_html(`<!> <button>break</button>`, 1);

export default function Boundary($$anchor, $$props) {
	$.push($$props, true);

	let broken = $.state(false);

	function explode() {
		if ($.get(broken)) throw new Error('boom');

		return 'fine';
	}

	var fragment = root_2();
	var node = $.first_child(fragment);

	{
		const failed = ($$anchor, error = $.noop, reset = $.noop) => {
			var p = root();
			var text = $.only_child(p, true);

			$.template_effect(() => $.set_text(text, error().message));
			$.append($$anchor, p);
		};

		$.boundary(node, { failed }, ($$anchor) => {
			var p_1 = root_1();
			var text_1 = $.only_child(p_1, true);

			$.template_effect(($0) => $.set_text(text_1, $0), [() => explode()]);
			$.append($$anchor, p_1);
		});
	}

	var button = $.sibling(node, 2);

	$.delegated('click', button, () => $.set(broken, true));
	$.append($$anchor, fragment);
	$.pop();
}

$.delegate(['click']);