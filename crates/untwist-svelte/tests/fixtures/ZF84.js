import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button class="go">go</button> <pre>  <!></pre>`, 1);

export default function ZF84($$anchor) {
	let a = $.state('A');
	let b = $.state('');
	let on = $.state(true);
	let items = $.state($.proxy([1, 2]));
	let h = '<u>u</u>';

	function t() {
		$.set(a, $.get(a) + '!');
		$.set(b, $.get(b) ? '' : 'Q', true);
		$.set(on, !$.get(on));
		$.set(items, [...$.get(items), 3], true);
	}

	var fragment = root();
	var button = $.first_child(fragment);
	var pre = $.sibling(button, 2);
	var node = $.sibling($.child(pre));

	{
		var consequent = ($$anchor) => {
			var text = $.text('Y');

			$.append($$anchor, text);
		};

		$.if(node, ($$render) => {
			if ($.get(on)) $$render(consequent);
		});
	}

	$.reset(pre);
	$.delegated('click', button, t);
	$.append($$anchor, fragment);
}

$.delegate(['click']);