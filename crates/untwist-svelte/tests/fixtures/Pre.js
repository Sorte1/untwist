import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(
	`<button>b</button> <pre> </pre> <pre>

two</pre> <pre>static
  only </pre> <textarea></textarea> <p> <b>bold</b> tail</p> <p> </p> <span></span><em></em> <p> </p><p> </p> <div>ab</div>`,
	1
);

export default function Pre($$anchor) {
	let a = $.state('x');
	var fragment = root();
	var button = $.first_child(fragment);
	var pre = $.sibling(button, 2);
	var text = $.only_child(pre);
	var textarea = $.sibling(pre, 6);

	$.remove_textarea_child(textarea);

	var p = $.sibling(textarea, 2);
	var text_1 = $.child(p);

	$.next(2);
	$.reset(p);

	var p_1 = $.sibling(p, 2);
	var text_2 = $.only_child(p_1);
	var em = $.sibling(p_1, 3);

	em.textContent = ' ';

	var p_2 = $.sibling(em, 2);
	var text_3 = $.only_child(p_2, true);
	var p_3 = $.sibling(p_2);
	var text_4 = $.only_child(p_3, true);

	$.next(2);

	$.template_effect(() => {
		$.set_text(text, `
  first
    ${$.get(a) ?? ''}
`);

		$.set_value(textarea, ` lead ${$.get(a) ?? ''}`);
		$.set_text(text_1, `${$.get(a) ?? ''} `);
		$.set_text(text_2, `x ${$.get(a) ?? ''}  y   z`);
		$.set_text(text_3, $.get(a));
		$.set_text(text_4, $.get(a));
	});

	$.delegated('click', button, () => $.set(a, $.get(a) + 'y'));
	$.append($$anchor, fragment);
}

$.delegate(['click']);