//! Domain expansion — generates all constraint combinations from field definitions.
//!
//! Each combination is a value vector (one value per field) forming the
//! cartesian product of all field domains.

use crate::spec::{ConstraintSpec, VerificationSpec};
use tagma_core::{Coord, CoordPath};

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
pub const MAX_COMBINATIONS: usize = 1_000_000_000; // 1B — CoordSpace removes Vec allocation

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

    let field_names: Vec<&String> = spec.fields.keys().collect();

    // Apply enable_mask constraints: force specific fields to zero
    // when their trigger condition is met.
    for constraint in &spec.constraints {
        if let ConstraintSpec::EnableMask {
            field,
            value,
            disable,
        } = constraint
        {
            let trigger_axis = field_names.iter().position(|n| *n == field);
            let disable_axes: Vec<usize> = disable
                .iter()
                .filter_map(|d| field_names.iter().position(|n| *n == d))
                .collect();

            if let Some(trigger_axis) = trigger_axis {
                for combo in combinations.iter_mut() {
                    if combo.values[trigger_axis] == *value {
                        for &axis in &disable_axes {
                            combo.values[axis] = 0;
                        }
                    }
                }
            }
        }
    }

    // Apply enable_set constraints: force specific fields to specified values
    // when their trigger condition is met.
    for constraint in &spec.constraints {
        if let ConstraintSpec::EnableSet { field, value, set } = constraint {
            let trigger_axis = field_names.iter().position(|n| *n == field);
            if let Some(trigger_axis) = trigger_axis {
                for combo in combinations.iter_mut() {
                    if combo.values[trigger_axis] == *value {
                        for assignment in set {
                            if let Some(axis) =
                                field_names.iter().position(|n| *n == &assignment.field)
                            {
                                combo.values[axis] = assignment.value;
                            }
                        }
                    }
                }
            }
        }
    }

    // Rebuild coordinates and point after mask application
    for combo in combinations.iter_mut() {
        combo.coordinates = Coordinates::new(combo.values.clone());
        combo.point = Point::new(combo.coordinates.clone());
    }

    Ok(combinations)
}

/// Convert a coordinate slice to a Vec<Coord> for DynCoordSpace indexing.
///
/// Returns None if any field value exceeds the u16 Coord range.
pub fn coords_to_coord_vec(coords: &[i64]) -> Option<Vec<Coord>> {
    let mut result = Vec::with_capacity(coords.len());
    for &v in coords {
        let c = Coord::new(v as u16)?;
        result.push(c);
    }
    Some(result)
}

/// Convert a coordinate vector to a CoordPath for CoordSpace indexing.
///
/// Returns None if any field value exceeds the u16 Coord range.
pub fn coords_to_path<const N: usize>(coords: &[i64]) -> Option<CoordPath<N>> {
    assert_eq!(coords.len(), N, "coords length must match CoordPath depth");
    let mut arr: [Coord; N] = [Coord::new(0).unwrap(); N];
    for (i, &v) in coords.iter().enumerate() {
        let c = Coord::new(v as u16)?;
        arr[i] = c;
    }
    Some(CoordPath::new(arr))
}

/// Build a structural filter from constraints.
///
/// Returns per-field allowed values where constraints structurally
/// restrict the space. Constraints that cannot be expressed as
/// structural filters (eq, neq) are returned for runtime evaluation.
pub fn structural_filters(
    spec: &VerificationSpec,
) -> (Vec<Vec<i64>>, Vec<(usize, usize, std::collections::HashMap<i64, Vec<i64>>)>) {
    use std::collections::HashMap;

    let field_names: Vec<&String> = spec.fields.keys().collect();
    let domains: Vec<Vec<i64>> = field_names
        .iter()
        .map(|name| {
            let def = spec.fields.get(*name).expect("field must exist");
            def.expand()
        })
        .collect();

    // Start with full domain per field
    let mut allowed: Vec<Vec<i64>> = domains.clone();
    // Cross constraints: (parent_idx, child_idx, mapping)
    let mut cross_maps: Vec<(usize, usize, HashMap<i64, Vec<i64>>)> = Vec::new();

    for constraint in &spec.constraints {
        match constraint {
            // oneof: restrict field to specific values
            ConstraintSpec::Oneof { field, values } => {
                if let Some(idx) = field_names.iter().position(|n| *n == field) {
                    // Intersect current allowed with the oneof values
                    let current = &allowed[idx];
                    let restricted: Vec<i64> = current
                        .iter()
                        .filter(|v| values.contains(v))
                        .copied()
                        .collect();
                    if !restricted.is_empty() {
                        allowed[idx] = restricted;
                    }
                }
            }
            // range: restrict field to [min, max]
            ConstraintSpec::Range { field, min, max } => {
                if let Some(idx) = field_names.iter().position(|n| *n == field) {
                    let current = &allowed[idx];
                    let restricted: Vec<i64> = current
                        .iter()
                        .filter(|v| **v >= *min && **v <= *max)
                        .copied()
                        .collect();
                    if !restricted.is_empty() {
                        allowed[idx] = restricted;
                    }
                }
            }
            // cross: field_a → field_b mapping
            // Do NOT restrict field_b's domain here — the restriction is
            // applied per-parent-value during enumeration via cross_maps.
            // When field_a is NOT in the mapping, field_b is unrestricted.
            ConstraintSpec::Cross {
                field_a,
                field_b,
                mapping,
            } => {
                let idx_a = field_names.iter().position(|n| *n == field_a);
                let idx_b = field_names.iter().position(|n| *n == field_b);
                if let (Some(ia), Some(ib)) = (idx_a, idx_b) {
                    cross_maps.push((ia, ib, mapping.clone()));
                }
            }
            // bitmask: field & mask == value → filter field values
            ConstraintSpec::Bitmask { field, mask, value } => {
                if let Some(idx) = field_names.iter().position(|n| *n == field) {
                    let current = &allowed[idx];
                    let restricted: Vec<i64> = current
                        .iter()
                        .filter(|v| (**v & *mask) == *value)
                        .copied()
                        .collect();
                    if !restricted.is_empty() {
                        allowed[idx] = restricted;
                    }
                }
            }
            // eq, neq, lt, gt, le, ge, even: runtime only
            _ => {}
        }
    }

    // Also apply field-level range restrictions from FieldSpec
    for (i, name) in field_names.iter().enumerate() {
        let def = spec.fields.get(*name).expect("field must exist");
        let restricted: Vec<i64> = allowed[i]
            .iter()
            .filter(|v| def.allows(**v))
            .copied()
            .collect();
        if !restricted.is_empty() {
            allowed[i] = restricted;
        }
    }

    (allowed, cross_maps)
}

/// A structural iterator that generates only valid combinations
/// by encoding constraints into the enumeration space.
pub struct StructuralEnum {
    /// Per-field allowed values after applying structural constraints.
    allowed: Vec<Vec<i64>>,
    /// Current index into each field's allowed values.
    indices: Vec<usize>,
    /// Cross mapping: field_a_idx → (field_b_idx, mapping)
    /// When iterating field_b, use the current field_a value to
    /// restrict which field_b values are valid.
    cross_maps: std::collections::HashMap<usize, (usize, std::collections::HashMap<i64, Vec<i64>>)>,
    /// Done flag.
    done: bool,
    /// Field names (for enable_mask/ enable_set).
    field_names: Vec<String>,
    /// Constraints for enable processing at enumeration time.
    constraints: Vec<ConstraintSpec>,
}

impl StructuralEnum {
    pub fn new(spec: &VerificationSpec) -> Self {
        let (allowed, cross_maps) = structural_filters(spec);
        let field_names: Vec<String> = spec.fields.keys().cloned().collect();

        // Re-index cross maps: for each child field, store a reference
        // to its parent field and mapping
        let mut indexed: std::collections::HashMap<usize, (usize, std::collections::HashMap<i64, Vec<i64>>)> =
            std::collections::HashMap::new();
        for (parent, child, mapping) in cross_maps {
            indexed.insert(child, (parent, mapping));
        }

        let n = allowed.len();
        Self {
            allowed,
            indices: vec![0; n],
            cross_maps: indexed,
            done: n == 0,
            field_names,
            constraints: spec.constraints.clone(),
        }
    }

    fn advance_indices(&mut self, current_values: &mut Vec<i64>) -> bool {
        for i in (0..self.indices.len()).rev() {
            // For cross-constrained fields, we may need to try multiple
            // values until we find one that satisfies the constraint.
            loop {
                self.indices[i] += 1;
                if self.indices[i] >= self.allowed[i].len() {
                    self.indices[i] = 0;
                    current_values[i] = self.allowed[i][0];
                    break; // carry to next higher field
                }
                current_values[i] = self.allowed[i][self.indices[i]];

                // Check cross constraint, if any
                let cross_ok = if let Some((parent_idx, mapping)) = self.cross_maps.get(&i) {
                    let parent_val = current_values[*parent_idx];
                    mapping
                        .get(&parent_val)
                        .map(|vals| vals.contains(&current_values[i]))
                        .unwrap_or(true) // not in mapping = unrestricted
                } else {
                    true
                };

                if cross_ok {
                    return false; // found a valid value, no carry
                }
                // else: try next value (continue inner loop)
            }
        }
        true // all fields carried = done
    }
}

impl Iterator for StructuralEnum {
    type Item = Combination;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // First iteration: use initial indices
        // Build current values
        let mut values: Vec<i64> = self
            .indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| self.allowed[i][idx])
            .collect();

        // Advance to next combination
        let carry = self.advance_indices(&mut values);
        if carry {
            self.done = true;
        }

        // Apply enable_mask / enable_set
        let final_values = self.apply_enable_masks(&values);

        let coordinates = Coordinates::new(final_values.clone());
        let point = Point::new(coordinates.clone());
        Some(Combination {
            values: final_values,
            coordinates,
            point,
        })
    }
}

impl StructuralEnum {
    fn apply_enable_masks(&self, values: &[i64]) -> Vec<i64> {
        let mut result = values.to_vec();
        for constraint in &self.constraints {
            match constraint {
                ConstraintSpec::EnableMask { field, value, disable } => {
                    if let Some(trigger_axis) = self.field_names.iter().position(|n| n == field) {
                        if result[trigger_axis] == *value {
                            for f in disable {
                                if let Some(axis) = self.field_names.iter().position(|n| n == f) {
                                    result[axis] = 0;
                                }
                            }
                        }
                    }
                }
                ConstraintSpec::EnableSet { field, value, set } => {
                    if let Some(trigger_axis) = self.field_names.iter().position(|n| n == field) {
                        if result[trigger_axis] == *value {
                            for assignment in set {
                                if let Some(axis) = self.field_names.iter().position(|n| n == &assignment.field) {
                                    result[axis] = assignment.value;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        result
    }

    /// Total number of structurally valid combinations.
    /// Returns None if the product overflows.
    pub fn total_combinations(&self) -> Option<usize> {
        let mut total: usize = 1;
        for i in 0..self.allowed.len() {
            total = total.checked_mul(self.allowed[i].len())?;
        }
        Some(total)
    }
}

/// Lazy cartesian product iterator over field domains.
///
/// Unlike `expand_all()`, this iterator does not pre-allocate a Vec.
/// Each combination is produced on demand, yielding O(1) memory
/// regardless of total combination count.
pub struct EnumerateIter {
    domains: Vec<Vec<i64>>,
    indices: Vec<usize>,
    done: bool,
    field_names: Vec<String>,
    constraints: Vec<ConstraintSpec>,
}

impl EnumerateIter {
    pub fn new(spec: &VerificationSpec) -> Self {
        let field_names: Vec<String> = spec.fields.keys().cloned().collect();
        let domains: Vec<Vec<i64>> = field_names
            .iter()
            .map(|name| {
                let def = spec.fields.get(name).expect("field must exist");
                def.expand()
            })
            .collect();
        let indices = vec![0usize; domains.len()];
        let done = domains.is_empty();
        Self {
            domains,
            indices,
            done,
            field_names,
            constraints: spec.constraints.clone(),
        }
    }
}

impl Iterator for EnumerateIter {
    type Item = Combination;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Build the current combination from current indices
        let values: Vec<i64> = self
            .indices
            .iter()
            .enumerate()
            .map(|(i, &idx)| self.domains[i][idx])
            .collect();

        // Advance to the next index vector
        let mut carry = true;
        for i in (0..self.indices.len()).rev() {
            if carry {
                self.indices[i] += 1;
                if self.indices[i] >= self.domains[i].len() {
                    self.indices[i] = 0;
                } else {
                    carry = false;
                }
            }
        }
        if carry {
            self.done = true;
        }

        // Apply enable_mask and enable_set constraints
        // (simplified: only handles basic case; full constraint processing
        //  is delegated to evaluate_all / validate_into_space)
        let final_values = self.apply_enable_masks(&values);

        let coordinates = Coordinates::new(final_values.clone());
        let point = Point::new(coordinates.clone());
        Some(Combination {
            values: final_values,
            coordinates,
            point,
        })
    }
}

impl EnumerateIter {
    fn apply_enable_masks(&self, values: &[i64]) -> Vec<i64> {
        let mut result = values.to_vec();
        for constraint in &self.constraints {
            match constraint {
                ConstraintSpec::EnableMask {
                    field,
                    value,
                    disable,
                } => {
                    if let Some(trigger_axis) =
                        self.field_names.iter().position(|n| n == field)
                    {
                        if result[trigger_axis] == *value {
                            for f in disable {
                                if let Some(axis) =
                                    self.field_names.iter().position(|n| n == f)
                                {
                                    result[axis] = 0;
                                }
                            }
                        }
                    }
                }
                ConstraintSpec::EnableSet {
                    field,
                    value,
                    set,
                } => {
                    if let Some(trigger_axis) =
                        self.field_names.iter().position(|n| n == field)
                    {
                        if result[trigger_axis] == *value {
                            for assignment in set {
                                if let Some(axis) =
                                    self.field_names.iter()
                                        .position(|n| n == &assignment.field)
                                {
                                    result[axis] = assignment.value;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        result
    }

    /// Total number of combinations (domain product).
    /// Returns None if the product overflows usize.
    pub fn total_combinations(&self) -> Option<usize> {
        let mut total: usize = 1;
        for d in &self.domains {
            total = total.checked_mul(d.len())?;
        }
        Some(total)
    }
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
    fn expand_enable_mask_forces_zero_on_trigger_match() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "op".into(),
            FieldSpec {
                range: Some((0, 3)),
                alignment: None,
                values: None,
            },
        );
        fields.insert(
            "rs1".into(),
            FieldSpec {
                range: Some((0, 3)),
                alignment: None,
                values: None,
            },
        );
        fields.insert(
            "rd".into(),
            FieldSpec {
                range: Some((0, 3)),
                alignment: None,
                values: None,
            },
        );
        let spec = VerificationSpec {
            target: "test".into(),
            fields,
            encoding: None,
            constraints: vec![crate::spec::ConstraintSpec::EnableMask {
                field: "op".into(),
                value: 0,
                disable: vec!["rs1".into(), "rd".into()],
            }],
            projector: crate::spec::ProjectorSpec::Sum,
        };
        let combos = expand_all(&spec).unwrap();
        // op ∈ {0,1,2,3}, rs1 ∈ {0,1,2,3}, rd ∈ {0,1,2,3} = 4×4×4 = 64
        assert_eq!(combos.len(), 64);
        // When op=0, rs1 and rd must be 0
        for combo in &combos {
            if combo.values[0] == 0 {
                assert_eq!(
                    combo.values[1], 0,
                    "when op=0, rs1 must be 0, got {:?}",
                    combo.values
                );
                assert_eq!(
                    combo.values[2], 0,
                    "when op=0, rd must be 0, got {:?}",
                    combo.values
                );
            }
        }
    }

    #[test]
    fn expand_enable_mask_no_trigger_leaves_values_unchanged() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "op".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![1, 2]),
            },
        );
        fields.insert(
            "rs1".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![1, 2]),
            },
        );
        let spec = VerificationSpec {
            target: "test".into(),
            fields,
            encoding: None,
            constraints: vec![crate::spec::ConstraintSpec::EnableMask {
                field: "op".into(),
                value: 0,
                disable: vec!["rs1".into()],
            }],
            projector: crate::spec::ProjectorSpec::Sum,
        };
        let combos = expand_all(&spec).unwrap();
        // 2×2 = 4, and trigger value 0 never appears
        assert_eq!(combos.len(), 4);
        // All rs1 values should be unchanged (1 or 2)
        for combo in &combos {
            assert!(
                combo.values[1] == 1 || combo.values[1] == 2,
                "rs1 must be 1 or 2 when op != 0, got {:?}",
                combo.values
            );
        }
    }

    #[test]
    fn expand_enable_mask_multiple_triggers() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "op".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2]),
            },
        );
        fields.insert(
            "rs1".into(),
            FieldSpec {
                range: Some((0, 7)),
                alignment: None,
                values: None,
            },
        );
        fields.insert(
            "rd".into(),
            FieldSpec {
                range: Some((0, 7)),
                alignment: None,
                values: None,
            },
        );
        let spec = VerificationSpec {
            target: "test".into(),
            fields,
            encoding: None,
            constraints: vec![
                crate::spec::ConstraintSpec::EnableMask {
                    field: "op".into(),
                    value: 0,
                    disable: vec!["rs1".into(), "rd".into()],
                },
                crate::spec::ConstraintSpec::EnableMask {
                    field: "op".into(),
                    value: 1,
                    disable: vec!["rd".into()],
                },
            ],
            projector: crate::spec::ProjectorSpec::Sum,
        };
        let combos = expand_all(&spec).unwrap();
        assert_eq!(combos.len(), 3 * 8 * 8); // 192
                                             // BTreeMap key order: op, rd, rs1
        for combo in &combos {
            match combo.values[0] {
                0 => {
                    // enable_mask for op=0 disables both rd (idx 1) and rs1 (idx 2)
                    assert!(
                        combo.values[1] == 0 && combo.values[2] == 0,
                        "op=0: rd and rs1 must be 0, got {:?}",
                        combo.values
                    );
                }
                1 => {
                    // enable_mask for op=1 disables only rd (idx 1)
                    assert!(
                        combo.values[1] == 0,
                        "op=1: rd must be 0, got {:?}",
                        combo.values
                    );
                }
                _ => {} // op=2: no restriction
            }
        }
    }

    #[test]
    fn expand_enable_set_forces_value_on_trigger_match() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "op".into(),
            FieldSpec {
                range: None,
                alignment: None,
                values: Some(vec![0, 1, 2]),
            },
        );
        fields.insert(
            "rs1".into(),
            FieldSpec {
                range: Some((0, 7)),
                alignment: None,
                values: None,
            },
        );
        fields.insert(
            "rd".into(),
            FieldSpec {
                range: Some((0, 7)),
                alignment: None,
                values: None,
            },
        );
        let spec = VerificationSpec {
            target: "test".into(),
            fields,
            encoding: None,
            constraints: vec![crate::spec::ConstraintSpec::EnableSet {
                field: "op".into(),
                value: 0,
                set: vec![
                    crate::spec::FieldAssignment {
                        field: "rs1".into(),
                        value: 5,
                    },
                    crate::spec::FieldAssignment {
                        field: "rd".into(),
                        value: 3,
                    },
                ],
            }],
            projector: crate::spec::ProjectorSpec::Sum,
        };
        let combos = expand_all(&spec).unwrap();
        // op ∈ {0,1,2}, rs1 ∈ {0..7}, rd ∈ {0..7} = 3 × 8 × 8 = 192
        assert_eq!(combos.len(), 192);
        // BTreeMap key order: op, rd, rs1
        // When op=0, rd must be 3 and rs1 must be 5
        // When op≠0, rd and rs1 are unrestricted
        for combo in &combos {
            match combo.values[0] {
                0 => {
                    assert_eq!(
                        combo.values[1], 3,
                        "when op=0, rd must be 3, got {:?}",
                        combo.values
                    );
                    assert_eq!(
                        combo.values[2], 5,
                        "when op=0, rs1 must be 5, got {:?}",
                        combo.values
                    );
                }
                1 | 2 => {
                    // rd and rs1 are unrestricted when op≠0
                    assert!(
                        combo.values[1] >= 0 && combo.values[1] <= 7,
                        "rd out of range when op={}, got {:?}",
                        combo.values[0],
                        combo.values
                    );
                    assert!(
                        combo.values[2] >= 0 && combo.values[2] <= 7,
                        "rs1 out of range when op={}, got {:?}",
                        combo.values[0],
                        combo.values
                    );
                }
                _ => unreachable!(),
            }
        }
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
