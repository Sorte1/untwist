import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';
import { fade, fly } from 'svelte/transition';

var root = $.from_html(`<p>both</p> <p>split</p>`, 1);
var root_1 = $.from_html(`<!> <button>toggle</button>`, 1);

export default function Transition($$anchor) {
	let show = $.state(true);
	var fragment = root_1();
	var node = $.first_child(fragment);

	{
		var consequent = ($$anchor) => {
			var fragment_1 = root();
			var p = $.first_child(fragment_1);
			var p_1 = $.sibling(p, 2);

			$.transition(3, p, () => fade, () => ({ duration: 0 }));
			$.transition(1, p_1, () => fly, () => ({ y: 10, duration: 0 }));
			$.transition(6, p_1, () => fade, () => ({ duration: 0 }));
			$.append($$anchor, fragment_1);
		};

		$.if(node, ($$render) => {
			if ($.get(show)) $$render(consequent);
		});
	}

	var button = $.sibling(node, 2);

	$.delegated('click', button, () => $.set(show, !$.get(show)));
	$.append($$anchor, fragment);
}

$.delegate(['click']);