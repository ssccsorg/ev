//! Domain expansion — generates all constraint combinations from field definitions.
//!
//! Each combination is a coordinate vector (one value per field) forming the
//! cartesian product of all field domains.

use crate::xif::XifDocument;
use ssccs_core::{Coordinates, Segment};

/// A single constraint combination — one coordinate in the verification space.
#[derive(Debug, Clone)]
pub struct Combination {
    /// Field values in the same order as `field_names()`.
    pub values: Vec<i64>,
    /// Corresponding coordinate in the abstract space.
    pub coordinates: Coordinates,
    /// Corresponding segment.
    pub segment: Segment,
}

/// Expand all field domains into the full cartesian product.
///
/// Returns combinations in deterministic order (lexicographic by field name,
/// then by value).
pub fn expand_all(doc: &XifDocument) -> Vec<Combination> {
    let names = doc.field_names();
    let domains: Vec<Vec<i64>> = names
        .iter()
        .map(|name| {
            let def = doc.fields.get(*name).expect("field must exist");
            def.expand()
        })
        .collect();

    if domains.is_empty() {
        return Vec::new();
    }

    let total: usize = domains.iter().map(|d| d.len()).product();
    let mut combinations = Vec::with_capacity(total);

    // Iterative cartesian product using index tracking.
    let mut indices = vec![0usize; domains.len()];
    loop {
        let values: Vec<i64> = indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| domains[i][idx])
            .collect();
        let coordinates = Coordinates::new(values.clone());
        let segment = Segment::new(coordinates.clone());
        combinations.push(Combination {
            values,
            coordinates,
            segment,
        });

        // Advance indices like an odometer.
        let mut carry = true;
        for i in (0..indices.len()).rev() {
            if carry {
                indices[i] += 1;
                if indices[i] >= domains[i].len() {
                    indices[i] = 0;
                    carry = true;
                } else {
                    carry = false;
                }
            }
        }
        if carry {
            break; // All combinations generated.
        }
    }

    combinations
}
