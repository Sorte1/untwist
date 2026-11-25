import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<em> </em>`);
var root_1 = $.from_html(`<!> <button>bump</button>`, 1);

export default function KeyBlock($$anchor) {
	let version = $.state(1);
	var fragment = root_1();
	var node = $.first_child(fragment);

	$.key(node, () => $.get(version), ($$anchor) => {
		var em = root();
		var text = $.only_child(em);

		$.template_effect(() => $.set_text(text, `v${$.get(version) ?? ''}`));
		$.append($$anchor, em);
	});

	var button = $.sibling(node, 2);

	$.delegated('click', button, () => $.update(version));
	$.append($$anchor, fragment);
}

$.delegate(['click']);