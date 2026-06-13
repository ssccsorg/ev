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

/// Maximum number of combinations expand_all will generate before returning an error.
///
/// Set to 10 million to cover typical RISC-V XIF designs (4-6 fields, small domains)
/// while preventing OOM from accidentally large specifications.
pub const MAX_COMBINATIONS: usize = 34_000_000;

/// Expand all field domains into the full cartesian product.
///
/// Returns an error if the total number of combinations exceeds `MAX_COMBINATIONS` or
/// if the product computation overflows `usize`. This prevents accidentally requesting
/// an infeasibly large search space.
pub fn expand_all(spec: &VerificationSpec) -> Result<Vec<Combination>, String> {
    let names: Vec<&String> = spec.fields.keys().collect();
    let domains: Vec<Vec<i64>> = names
        .iter()
        .map(|name| {
            let def = spec.fields.get(*name).expect("field must exist");
            def.expand()
        })
        .collect();

    if domains.is_empty() {
        return Ok(Vec::new());
    }

    // Check for overflow and upper bound before allocating.
    let mut total: usize = 1;
    for d in &domains {
        let len = d.len();
        total = total.checked_mul(len).ok_or_else(|| {
            format!(
                "domain expansion overflow: total combinations exceed usize (field domain size {})",
                len
            )
        })?;
        if total > MAX_COMBINATIONS {
            return Err(format!(
                "domain expansion too large: {} combinations exceed limit of {}. \
                 Reduce field domain sizes or increase MAX_COMBINATIONS.",
                total, MAX_COMBINATIONS
            ));
        }
    }

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

    Ok(combinations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::FieldSpec;
    use std::collections::BTreeMap;

    fn make_spec(fields: BTreeMap<String, FieldSpec>) -> VerificationSpec {
        VerificationSpec {
            target: "test".into(),
            fields,
            encoding: None,
            constraints: vec![],
            projector: crate::spec::ProjectorSpec::Sum,
        }
    }

    #[test]
    fn expand_single_field_two_values() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![2, 4]),
            },
        );
        let spec = make_spec(fields);
        let combos = expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 2, "2 values = 2 combos");
        assert_eq!(combos[0].values, vec![2]);
        assert_eq!(combos[1].values, vec![4]);
    }

    #[test]
    fn expand_two_fields_cartesian_product() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "a".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![1, 2]),
            },
        );
        fields.insert(
            "b".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![10, 20, 30]),
            },
        );
        let spec = make_spec(fields);
        let combos = expand_all(&spec).unwrap();
        // 2 * 3 = 6 combinations
        assert_eq!(combos.len(), 6);
        // First combination: a=1, b=10
        assert_eq!(combos[0].values, vec![1, 10]);
        // Last combination: a=2, b=30
        assert_eq!(combos[5].values, vec![2, 30]);
        // All combinations are unique
        let mut uniq = std::collections::HashSet::new();
        for c in &combos {
            assert!(
                uniq.insert(c.values.clone()),
                "duplicate combo: {:?}",
                c.values
            );
        }
    }

    #[test]
    fn expand_range_field() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "n".into(),
            FieldSpec {
                range: Some((0, 3)),
                alignment: None,
                values: None,
            },
        );
        let spec = make_spec(fields);
        let combos = expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 4, "0..=3 = 4 values");
        assert_eq!(combos[0].values, vec![0]);
        assert_eq!(combos[3].values, vec![3]);
    }

    #[test]
    fn expand_empty_fields_returns_empty() {
        let spec = make_spec(BTreeMap::new());
        let combos = expand_all(&spec).unwrap();
        assert!(combos.is_empty());
    }

    #[test]
    fn expand_alignment_step() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "v".into(),
            FieldSpec {
                range: Some((0, 10)),
                alignment: Some(3),
                values: None,
            },
        );
        let spec = make_spec(fields);
        let combos = expand_all(&spec).unwrap();
        // 0, 3, 6, 9
        assert_eq!(combos.len(), 4);
        assert_eq!(combos[0].values, vec![0]);
        assert_eq!(combos[1].values, vec![3]);
        assert_eq!(combos[2].values, vec![6]);
        assert_eq!(combos[3].values, vec![9]);
    }

    #[test]
    fn expand_preserves_point_coordinates() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "x".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![5]),
            },
        );
        let spec = make_spec(fields);
        let combos = expand_all(&spec).unwrap();
        assert_eq!(combos[0].point.coordinates().raw, vec![5]);
        assert_eq!(combos[0].coordinates.raw, vec![5]);
    }

    #[test]
    fn expand_exceeding_max_combinations_returns_error() {
        // 11 small fields = 2^11 = 2048 (fine), but 10M + 1
        // We override MAX_COMBINATIONS by using a large value domain.
        let mut fields = BTreeMap::new();
        // One field with range 0..=MAX_COMBINATIONS (forces overflow)
        fields.insert(
            "big".into(),
            FieldSpec {
                range: Some((0, MAX_COMBINATIONS as i64)),
                alignment: None,
                values: None,
            },
        );
        let spec = make_spec(fields);
        let result = expand_all(&spec);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("too large") || err.contains("limit"),
            "error should mention limit: {}",
            err
        );
    }

    #[test]
    fn expand_exceeding_max_combinations_large_product() {
        // 10 fields each with 8 values => 8^10 = 1_073_741_824 > MAX_COMBINATIONS.
        // This triggers the MAX_COMBINATIONS guard, not overflow.
        let mut fields = BTreeMap::new();
        for i in 0..10 {
            fields.insert(
                format!("f{}", i),
                FieldSpec {
                    range: Some((0, 7)),
                    alignment: None,
                    values: None,
                },
            );
        }
        let spec = make_spec(fields);
        let result = expand_all(&spec);
        assert!(
            result.is_err(),
            "10 fields of 8 values should exceed MAX_COMBINATIONS"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("too large") || err.contains("limit"),
            "error should mention limit: {}",
            err
        );
    }
}
