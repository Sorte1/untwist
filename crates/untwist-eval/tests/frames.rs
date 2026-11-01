use std::rc::Rc;

use untwist_eval::budget::BINDING_BYTES;
use untwist_eval::env::{
    assign, child_env, declare, root_env, Assigned, ScopeChainTooDeep, SCOPE_CHAIN_CAP,
};
use untwist_eval::frame::{Frame, FrameBody, FrameLayout, FrameRef, SlotKind};
use untwist_eval::heap::{reserve_lenient, HeapLedger};
use untwist_eval::names::{Name, Names};
use untwist_eval::value::Value;

fn layout(
    names: &mut Names<'static>,
    count: usize,
    kind: SlotKind,
) -> (Rc<FrameLayout>, Vec<Name>) {
    let slots: Vec<Name> = (0..count)
        .map(|index| names.intern(Box::leak(format!("s{index}").into_boxed_str())))
        .collect();
    let built = FrameLayout::new(slots.iter().map(|name| (*name, kind)).collect())
        .expect("a layout under u32::MAX slots");
    (Rc::new(built), slots)
}

fn number(frame: &FrameRef<'static>, name: Name) -> Option<f64> {
    match Frame::slot_by_name(frame, name) {
        Ok(Some(Value::Number(n))) => Some(n),
        _ => None,
    }
}

fn slot_count(frame: &FrameRef<'static>) -> usize {
    match &frame.borrow().body {
        FrameBody::Fixed { slots, .. } => slots.len(),
        FrameBody::Dynamic(_) => 0,
    }
}

#[test]
fn an_undeclared_fixed_slot_is_absent_so_a_read_goes_outward() {
    let mut names = Names::default();
    let (shared, slots) = layout(&mut names, 3, SlotKind::Mutable);
    let root = root_env();
    declare(&root, slots[1], Value::Number(7.0));
    let frame = Frame::fixed(&shared, Some(&root), None);
    assert_eq!(number(&frame, slots[1]), Some(7.0));
    Frame::declare(&frame, slots[1], Value::Number(1.0));
    assert_eq!(number(&frame, slots[1]), Some(1.0));
    assert_eq!(number(&root, slots[1]), Some(7.0));
    assert_eq!(frame.borrow().body.len(), 1);
    assert!(matches!(Frame::slot_by_name(&frame, slots[0]), Ok(None)));
}

#[test]
fn a_fixed_frame_charges_one_binding_per_newly_declared_slot_as_a_dynamic_one_does() {
    let mut names = Names::default();
    let (shared, slots) = layout(&mut names, 2, SlotKind::Mutable);
    let ledger = HeapLedger::new(1 << 20);
    let fixed = Frame::fixed(&shared, None, Some(reserve_lenient(&ledger, 0)));
    let dynamic = Frame::dynamic(None, Some(reserve_lenient(&ledger, 0)));
    for frame in [&fixed, &dynamic] {
        Frame::declare(frame, slots[0], Value::Number(1.0));
        Frame::declare(frame, slots[0], Value::Number(2.0));
        Frame::declare_if_absent(frame, slots[1], Value::Null);
        Frame::declare_if_absent(frame, slots[1], Value::Undefined);
    }
    assert_eq!(ledger.live(), 4 * BINDING_BYTES);
    assert_eq!(number(&fixed, slots[0]), Some(2.0));
    assert!(matches!(
        Frame::slot_by_name(&fixed, slots[1]),
        Ok(Some(Value::Null))
    ));
}

#[test]
fn past_sixty_four_slots_the_declared_bits_live_in_the_same_allocation() {
    let mut names = Names::default();
    let (shared, slots) = layout(&mut names, 130, SlotKind::Mutable);
    let frame = Frame::fixed(&shared, None, None);
    assert_eq!(slot_count(&frame), 130 + 3);
    for index in [0, 63, 64, 95, 96, 129] {
        assert_eq!(number(&frame, slots[index]), None, "slot {index}");
        Frame::declare(&frame, slots[index], Value::Number(index as f64));
    }
    for index in [0, 63, 64, 95, 96, 129] {
        assert_eq!(
            number(&frame, slots[index]),
            Some(index as f64),
            "slot {index}"
        );
    }
    assert_eq!(number(&frame, slots[100]), None);
    assert_eq!(frame.borrow().body.len(), 6);
    frame.borrow_mut().body.clear();
    assert_eq!(frame.borrow().body.len(), 0);
    assert_eq!(number(&frame, slots[129]), None);
}

#[test]
fn an_immutable_fixed_slot_refuses_the_write_and_keeps_its_value() {
    let mut names = Names::default();
    let (shared, slots) = layout(&mut names, 1, SlotKind::Immutable);
    let named = Frame::fixed(&shared, None, None);
    Frame::declare_immutable(&named, slots[0], Value::Number(1.0));
    let inner = child_env(&named);
    assert_eq!(
        assign(&inner, slots[0], Value::Number(2.0)),
        Ok(Assigned::Immutable)
    );
    assert_eq!(number(&inner, slots[0]), Some(1.0));
}

#[test]
fn a_by_name_read_through_fixed_frames_keeps_the_hop_cap() {
    let mut names = Names::default();
    let (shared, slots) = layout(&mut names, 1, SlotKind::Mutable);
    let root = root_env();
    declare(&root, slots[0], Value::Number(3.0));
    let mut chain = root.clone();
    for _ in 0..SCOPE_CHAIN_CAP - 1 {
        chain = Frame::fixed(&shared, Some(&chain), None);
    }
    assert_eq!(number(&chain, slots[0]), Some(3.0));
    let beyond = Frame::fixed(&shared, Some(&chain), None);
    assert!(Frame::slot_by_name(&beyond, slots[0]).is_err());
    assert!(matches!(
        Frame::slot_by_name(&beyond, slots[0]),
        Err(ScopeChainTooDeep)
    ));
}

#[test]
fn a_wide_layout_finds_its_slots_through_its_index() {
    let mut names = Names::default();
    let (shared, slots) = layout(&mut names, 40, SlotKind::Mutable);
    let frame = Frame::fixed(&shared, None, None);
    for (index, name) in slots.iter().enumerate() {
        Frame::declare(&frame, *name, Value::Number(index as f64));
    }
    for (index, name) in slots.iter().enumerate() {
        assert_eq!(number(&frame, *name), Some(index as f64));
    }
    assert_eq!(shared.len(), 40);
}
