import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<li> </li>`);
var root_1 = $.from_html(`<ol></ol>`);
var root_2 = $.from_html(`<p>none</p>`);
var root_3 = $.from_html(`<section><h3> </h3> <!></section>`);
var root_4 = $.from_html(`<button>toggle</button> <!>`, 1);

export default function Nested($$anchor) {
	let groups = $.proxy([{ name: 'g1', items: ['a', 'b'] }, { name: 'g2', items: [] }]);
	let open = $.state(true);
	var fragment = root_4();
	var button = $.first_child(fragment);
	var node = $.sibling(button, 2);

	{
		var consequent_1 = ($$anchor) => {
			var fragment_1 = $.comment();
			var node_1 = $.first_child(fragment_1);

			$.each(node_1, 17, () => groups, $.index, ($$anchor, group) => {
				var section = root_3();
				var h3 = $.child(section);
				var text = $.only_child(h3, true);
				var node_2 = $.sibling(h3, 2);

				{
					var consequent = ($$anchor) => {
						var ol = root_1();

						$.each(ol, 21, () => $.get(group).items, $.index, ($$anchor, item) => {
							var li = root();
							var text_1 = $.only_child(li, true);

							$.template_effect(() => $.set_text(text_1, $.get(item)));
							$.append($$anchor, li);
						});

						$.reset(ol);
						$.append($$anchor, ol);
					};

					var alternate = ($$anchor) => {
						var p = root_2();

						$.append($$anchor, p);
					};

					$.if(node_2, ($$render) => {
						if ($.get(group).items.length) $$render(consequent); else $$render(alternate, -1);
					});
				}

				$.reset(section);
				$.template_effect(() => $.set_text(text, $.get(group).name));
				$.append($$anchor, section);
			});

			$.append($$anchor, fragment_1);
		};

		$.if(node, ($$render) => {
			if ($.get(open)) $$render(consequent_1);
		});
	}

	$.delegated('click', button, () => $.set(open, !$.get(open)));
	$.append($$anchor, fragment);
}

$.delegate(['click']);