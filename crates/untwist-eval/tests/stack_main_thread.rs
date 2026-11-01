mod boundary;

use boundary::{assert_at_the_grow_boundary, descend_through_segments};

fn main() {
    let reached = descend_through_segments(0, 2);
    assert_eq!(reached.len(), 3, "{reached:?}");
    reached.iter().for_each(assert_at_the_grow_boundary);
    println!("the main thread and two grown segments reached their grow boundaries: {reached:?}");
}
