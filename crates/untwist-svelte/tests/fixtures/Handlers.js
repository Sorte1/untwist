import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<li> </li>`);
var root_1 = $.from_html(`<button>x</button> <div><span>inner</span></div> <ul></ul>`, 1);

export default function Handlers($$anchor) {
	let seen = $.proxy([]);

	function record(event, tag) {
		seen.push(tag + ':' + event.type);
	}

	var fragment = root_1();
	var button = $.first_child(fragment);
	var div = $.sibling(button, 2);
	var ul = $.sibling(div, 2);

	$.each(ul, 21, () => seen, $.index, ($$anchor, item) => {
		var li = root();
		var text = $.only_child(li, true);

		$.template_effect(() => $.set_text(text, $.get(item)));
		$.append($$anchor, li);
	});

	$.reset(ul);
	$.delegated('click', button, (event) => record(event, 'a'));
	$.delegated('dblclick', button, (e) => record(e, 'b'));
	$.event('click', div, (e) => record(e, 'cap'), true);
	$.append($$anchor, fragment);
}

$.delegate(['click', 'dblclick']);