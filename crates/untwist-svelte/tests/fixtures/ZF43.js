import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<button class="go">go</button> <pre> <i> </i><!> </pre>`, 1);

export default function ZF43($$anchor) {
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
	var text = $.child(pre, true);

	text.nodeValue = '\n';

	var i = $.sibling(text);
	var text_1 = $.only_child(i, true);
	var node = $.sibling(i);

	{
		var consequent = ($$anchor) => {
			var text_2 = $.text(' Y ');

			$.append($$anchor, text_2);
		};

		var alternate = ($$anchor) => {
			var text_3 = $.text(' N ');

			$.append($$anchor, text_3);
		};

		$.if(node, ($$render) => {
			if ($.get(on)) $$render(consequent); else $$render(alternate, -1);
		});
	}

	var text_4 = $.sibling(node, 1, true);

	text_4.nodeValue = '  ';
	$.reset(pre);
	$.template_effect(() => $.set_text(text_1, $.get(a)));
	$.delegated('click', button, t);
	$.append($$anchor, fragment);
}

$.delegate(['click']);