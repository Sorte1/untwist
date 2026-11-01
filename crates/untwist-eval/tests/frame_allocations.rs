use std::rc::Rc;

use untwist_eval::frame::{Frame, FrameBody, FrameLayout, SlotKind};
use untwist_eval::names::Names;
use untwist_eval::value::Value;

fn layout(names: &mut Names<'static>, source: &'static str) -> Rc<FrameLayout> {
    let slots = source
        .split_whitespace()
        .map(|text| (names.intern(text), SlotKind::Mutable))
        .collect();
    Rc::new(FrameLayout::new(slots).expect("a layout"))
}

fn source(width: usize) -> &'static str {
    let text: String = (0..width).map(|n| format!("v{n} ")).collect();
    Box::leak(text.into_boxed_str())
}

#[test]
fn a_fixed_frames_slots_are_one_block_of_its_width_and_tail_words() {
    let root = Frame::dynamic(None, None);
    for (width, tail) in [
        (0, 0),
        (1, 0),
        (12, 0),
        (64, 0),
        (65, 1),
        (96, 1),
        (97, 2),
        (130, 3),
    ] {
        let text = source(width);
        let mut names = Names::for_source(text);
        let layout = layout(&mut names, text);
        let frame = Frame::fixed(&layout, Some(&root), None);
        if let Some(first) = text.split_whitespace().next() {
            Frame::declare(&frame, names.intern(first), Value::Null);
        }
        let borrowed = frame.borrow();
        let FrameBody::Fixed { slots, .. } = &borrowed.body else {
            panic!("width {width}: a fixed frame");
        };
        assert_eq!(slots.len(), width + tail, "width {width}");
        assert!(Rc::ptr_eq(
            &frame.borrow().parent.clone().expect("a parent"),
            &root
        ));
        assert_eq!(
            Rc::strong_count(&layout),
            2,
            "width {width}: the layout is shared, not copied"
        );
    }
}
