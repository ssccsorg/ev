//! Domain expansion — generates all constraint combinations from field definitions.
//!
//! Each combination is a value vector (one value per field) forming the
//! cartesian product of all field domains.

use crate::spec::VerificationSpec;

/// A coordinate vector — one value per instruction field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Coordinates {
    pub raw: Vec<i64>,
}

impl Coordinates {
    pub fn new(raw: Vec<i64>) -> Self {
        Self { raw }
    }

    pub fn get_axis(&self, axis: usize) -> Option<i64> {
        self.raw.get(axis).copied()
    }
}

/// An immutable coordinate point.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Point {
    coord: Coordinates,
}

impl Point {
    pub fn new(coord: Coordinates) -> Self {
        Self { coord }
    }

    pub fn coordinates(&self) -> &Coordinates {
        &self.coord
    }
}

/// A single constraint combination — one coordinate in the verification space.
#[derive(Debug, Clone)]
pub struct Combination {
    pub values: Vec<i64>,
    pub coordinates: Coordinates,
    pub point: Point,
}

/// Expand all field domains into the full cartesian product.
pub fn expand_all(spec: &VerificationSpec) -> Vec<Combination> {
    let names: Vec<&String> = spec.fields.keys().collect();
    let domains: Vec<Vec<i64>> = names
        .iter()
        .map(|name| {
            let def = spec.fields.get(*name).expect("field must exist");
            def.expand()
        })
        .collect();

    if domains.is_empty() {
        return Vec::new();
    }

    let total: usize = domains.iter().map(|d| d.len()).product();
    let mut combinations = Vec::with_capacity(total);

    let mut indices = vec![0usize; domains.len()];
    loop {
        let values: Vec<i64> = indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| domains[i][idx])
            .collect();
        let coordinates = Coordinates::new(values.clone());
        let point = Point::new(coordinates.clone());
        combinations.push(Combination {
            values,
            coordinates,
            point,
        });

        let mut carry = true;
        for i in (0..indices.len()).rev() {
            if carry {
                indices[i] += 1;
                if indices[i] >= domains[i].len() {
                    indices[i] = 0;
                } else {
                    carry = false;
                }
            }
        }
        if carry {
            break;
        }
    }

    combinations
}
