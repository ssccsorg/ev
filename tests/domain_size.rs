//! The domain size is computed without materializing the domain, and it
//! matches the expansion exactly.

use ev::spec::FieldSpec;

fn range(min: i64, max: i64, alignment: Option<i64>) -> FieldSpec {
    FieldSpec {
        range: Some((min, max)),
        alignment,
        values: None,
    }
}

#[test]
fn len_matches_expand_for_ranges() {
    for (min, max, step) in [
        (0, 15, 1),
        (0, 15, 3),
        (-8, 8, 3),
        (-6, 6, 2),
        (0, 0, 1),
        (5, 4, 1),
    ] {
        let field = range(min, max, Some(step));
        assert_eq!(
            field.len(),
            field.expand().len(),
            "range {min}..={max} step {step}"
        );
    }
}

#[test]
fn len_of_values_is_the_list_length() {
    let field = FieldSpec {
        range: None,
        alignment: None,
        values: Some(vec![5, 2, 7]),
    };
    assert_eq!(field.len(), 3);
    assert_eq!(field.len(), field.expand().len());
}

#[test]
fn len_of_the_default_domain_is_256() {
    let field = FieldSpec {
        range: None,
        alignment: None,
        values: None,
    };
    assert_eq!(field.len(), 256);
    assert_eq!(field.len(), field.expand().len());
}

#[test]
fn an_empty_range_is_empty() {
    let field = range(5, 4, None);
    assert_eq!(field.len(), 0);
    assert!(field.is_empty());
}
