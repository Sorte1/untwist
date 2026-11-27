import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<span> </span>`);
var root_1 = $.from_html(`<p><em></em>|<b></b>|<i></i>|<u></u></p> <p></p> <div><!></div> <div></div> <div></div> <div><span>a</span> <span>b</span> <span>c</span>&#32;<span>d</span></div> <button>b</button>`, 1);

export default function Ws2($$anchor) {
	let a = 'x';
	let t = $.state(true);
	var fragment = root_1();
	var p = $.first_child(fragment);
	var em = $.child(p);

	em.textContent = ' ';

	var b = $.sibling(em, 2);

	b.textContent = 'x';

	var i = $.sibling(b, 2);

	i.textContent = 'x ';

	var u = $.sibling(i, 2);

	u.textContent = ' x';
	$.reset(p);

	var p_1 = $.sibling(p, 2);

	p_1.textContent = 'a\nx\nb';

	var div = $.sibling(p_1, 2);
	var node = $.child(div);

	{
		var consequent = ($$anchor) => {
			var text = $.text('yes');

			$.append($$anchor, text);
		};

		var alternate = ($$anchor) => {
			var text_1 = $.text('no');

			$.append($$anchor, text_1);
		};

		$.if(node, ($$render) => {
			if ($.get(t)) $$render(consequent); else $$render(alternate, -1);
		});
	}

	$.reset(div);

	var div_1 = $.sibling(div, 2);

	$.each(div_1, 20, () => [1, 2], $.index, ($$anchor, k) => {
		$.next();

		var text_2 = $.text();

		$.template_effect(() => $.set_text(text_2, k));
		$.append($$anchor, text_2);
	});

	$.reset(div_1);

	var div_2 = $.sibling(div_1, 2);

	$.each(div_2, 20, () => [1, 2], $.index, ($$anchor, k) => {
		var span = root();
		var text_3 = $.only_child(span, true);

		$.template_effect(() => $.set_text(text_3, k));
		$.append($$anchor, span);
	});

	$.reset(div_2);

	var div_3 = $.sibling(div_2, 2);
	var text_4 = $.sibling($.child(div_3), 1, true);

	text_4.nodeValue = ' ';
	$.next(5);
	$.reset(div_3);

	var button = $.sibling(div_3, 2);

	$.delegated('click', button, () => $.set(t, !$.get(t)));
	$.append($$anchor, fragment);
}

$.delegate(['click']);