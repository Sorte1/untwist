import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<b> </b>`);
var root_1 = $.from_html(`<pre>a <!> b</pre> <pre></pre> <div>k <!></div>`, 1);

export default function ZPreEach2($$anchor) {
	let items = $.proxy([1, 2]);
	let on = true;
	var fragment = root_1();
	var pre = $.first_child(fragment);
	var node = $.sibling($.child(pre));

	{
		var consequent = ($$anchor) => {
			var text = $.text('yes');

			$.append($$anchor, text);
		};

		$.if(node, ($$render) => {
			if (on) $$render(consequent);
		});
	}

	$.next();
	$.reset(pre);

	var pre_1 = $.sibling(pre, 2);

	$.each(pre_1, 21, () => items, $.index, ($$anchor, i) => {
		var b = root();
		var text_1 = $.only_child(b, true);

		$.template_effect(() => $.set_text(text_1, $.get(i)));
		$.append($$anchor, b);
	});

	$.reset(pre_1);

	var div = $.sibling(pre_1, 2);
	var node_1 = $.sibling($.child(div));

	$.each(node_1, 17, () => items, $.index, ($$anchor, i) => {
		$.next();

		var text_2 = $.text();

		$.template_effect(() => $.set_text(text_2, `${$.get(i) ?? ''},`));
		$.append($$anchor, text_2);
	});

	$.reset(div);
	$.append($$anchor, fragment);
}