import 'svelte/internal/disclose-version';
import * as $ from 'svelte/internal/client';

var root = $.from_html(`<li><input type="checkbox"/> <button class="remove">x</button></li>`);
var root_1 = $.from_html(`<input placeholder="new todo"/> <button class="add">add</button> <ul></ul> <p> </p>`, 1);

const $$css = {
	hash: 'svelte-70rljk',
	code: '.done.svelte-70rljk {text-decoration:line-through;}'
};

export default function Todo($$anchor, $$props) {
	$.push($$props, true);
	$.append_styles($$anchor, $$css);

	let todos = $.state($.proxy([{ id: 1, text: 'write', done: false }]));
	let draft = $.state('');
	let nextId = 2;
	let remaining = $.derived(() => $.get(todos).filter((todo) => !todo.done).length);

	function add() {
		if (!$.get(draft)) return;

		$.get(todos).push({ id: nextId++, text: $.get(draft), done: false });
		$.set(draft, '');
	}

	function remove(id) {
		$.set(todos, $.get(todos).filter((todo) => todo.id !== id), true);
	}

	var fragment = root_1();
	var input = $.first_child(fragment);

	$.remove_input_defaults(input);

	var button = $.sibling(input, 2);
	var ul = $.sibling(button, 2);

	$.each(ul, 21, () => $.get(todos), (todo) => todo.id, ($$anchor, todo, $$index) => {
		var li = root();
		let classes;
		var input_1 = $.child(li);

		$.remove_input_defaults(input_1);

		var text = $.sibling(input_1);
		var button_1 = $.sibling(text);

		$.reset(li);

		$.template_effect(() => {
			classes = $.set_class(li, 1, 'svelte-70rljk', null, classes, { done: $.get(todo).done });
			$.set_text(text, ` ${$.get(todo).text ?? ''} `);
		});

		$.bind_checked(input_1, () => $.get(todo).done, ($$value) => ($.get(todo).done = $$value));
		$.delegated('click', button_1, () => remove($.get(todo).id));
		$.append($$anchor, li);
	});

	$.reset(ul);

	var p = $.sibling(ul, 2);
	var text_1 = $.only_child(p);

	$.template_effect(() => $.set_text(text_1, `${$.get(remaining) ?? ''} left`));
	$.bind_value(input, () => $.get(draft), ($$value) => $.set(draft, $$value));
	$.delegated('click', button, add);
	$.append($$anchor, fragment);
	$.pop();
}

$.delegate(['click']);