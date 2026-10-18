//! The engine's coordinate space is its own type: distinct field-value
//! vectors, with no external coordinate crate.

use ev::verify::CoordSpace;

#[test]
fn placement_is_idempotent() {
    let mut space = CoordSpace::new();
    assert!(space.place(&[1, 2, 3]));
    assert!(!space.place(&[1, 2, 3]));
    assert_eq!(space.len(), 1);
}

#[test]
fn contains_distinguishes_coordinates() {
    let mut space = CoordSpace::new();
    space.place(&[0, 1]);

    assert!(space.contains(&[0, 1]));
    assert!(!space.contains(&[1, 0]));
    assert!(!space.contains(&[0]));
}

#[test]
fn values_outside_the_u16_range_are_kept() {
    let mut space = CoordSpace::new();
    assert!(space.place(&[-1, 70_000]));

    assert!(space.contains(&[-1, 70_000]));
    assert_eq!(space.iter().count(), 1);
}
