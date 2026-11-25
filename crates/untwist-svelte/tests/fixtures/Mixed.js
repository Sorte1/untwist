import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<p class="empty svelte-4qehul"> </p>`);
var root_1 = $.from_html(`<li class="svelte-4qehul"> </li>`);
var root_2 = $.from_html(`<ul class="svelte-4qehul"></ul>`);
var root_3 = $.from_html(`<section class="svelte-4qehul"><h2 class="svelte-4qehul"> </h2> <input placeholder="filter"/> <!></section>`);

const $$css = {
	hash: 'svelte-4qehul',
	code: 'section.svelte-4qehul > h2:where(.svelte-4qehul) {font-size:1.2em;}.empty.svelte-4qehul {color:gray;}ul.svelte-4qehul li:where(.svelte-4qehul):first-child {font-weight:bold;}'
};

export default function Mixed($$anchor, $$props) {
	$.push($$props, true);
	$.append_styles($$anchor, $$css);

	let title = $.prop($$props, 'title', 3, 'List'),
		items = $.prop($$props, 'items', 19, () => ['x', 'y']);

	let filter = $.state('');
	let shown = $.derived(() => items().filter((item) => item.includes($.get(filter))));
	var section = root_3();
	var h2 = $.child(section);
	var text = $.only_child(h2);
	var input = $.sibling(h2, 2);

	$.remove_input_defaults(input);

	var node = $.sibling(input, 2);

	{
		var consequent = ($$anchor) => {
			var p = root();
			var text_1 = $.only_child(p);

			$.template_effect(() => $.set_text(text_1, `nothing matches "${$.get(filter) ?? ''}"`));
			$.append($$anchor, p);
		};

		var alternate = ($$anchor) => {
			var ul = root_2();

			$.each(ul, 22, () => $.get(shown), (item) => item, ($$anchor, item, index) => {
				var li = root_1();
				var text_2 = $.only_child(li);

				$.template_effect(() => $.set_text(text_2, `${$.get(index) + 1}. ${item ?? ''}`));
				$.append($$anchor, li);
			});

			$.reset(ul);
			$.append($$anchor, ul);
		};

		$.if(node, ($$render) => {
			if ($.get(shown).length === 0) $$render(consequent); else $$render(alternate, -1);
		});
	}

	$.reset(section);
	$.template_effect(() => $.set_text(text, `${title() ?? ''} (${$.get(shown).length ?? ''})`));
	$.bind_value(input, () => $.get(filter), ($$value) => $.set(filter, $$value));
	$.append($$anchor, section);
	$.pop();
}