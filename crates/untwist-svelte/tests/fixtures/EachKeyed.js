import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<li> </li>`);
var root_1 = $.from_html(`<li>empty</li>`);
var root_2 = $.from_html(`<ul></ul> <button>drop</button>`, 1);

export default function EachKeyed($$anchor, $$props) {
	let items = $.prop($$props, 'items', 19, () => []);
	let list = $.state($.proxy(items()));

	function drop() {
		$.set(list, $.get(list).slice(1), true);
	}

	var fragment = root_2();
	var ul = $.first_child(fragment);

	$.each(
		ul,
		23,
		() => $.get(list),
		(item) => item.id,
		($$anchor, item, i) => {
			var li = root();
			var text = $.only_child(li, true);

			$.template_effect(() => {
				$.set_attribute(li, 'data-index', $.get(i));
				$.set_text(text, $.get(item).name);
			});

			$.append($$anchor, li);
		},
		($$anchor) => {
			var li_1 = root_1();

			$.append($$anchor, li_1);
		}
	);

	$.reset(ul);

	var button = $.sibling(ul, 2);

	$.delegated('click', button, drop);
	$.append($$anchor, fragment);
}

$.delegate(['click']);