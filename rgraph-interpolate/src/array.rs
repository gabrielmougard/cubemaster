//! Array-valued interpolators — port of d3-interpolate's `array.js`,
//! `numberArray.js`, and `object.js`.
//!
//! d3's array interpolators have two important quirks worth preserving:
//!
//! * If `a` and `b` are different lengths, the result has `b`'s length.
//!   The first `min(a.len, b.len)` slots are interpolated; the trailing
//!   slots are taken constant from `b`.
//! * If `a` is `None`/empty, the result is just `b` (constant).

use std::collections::HashMap;

/// Element-wise linear interpolation of two `f64` slices.
///
/// Equivalent to d3's `interpolateNumberArray(a, b)`. Returns a closure
/// that allocates a fresh `Vec<f64>` per call (matching d3, which mutates
/// a captured array; in Rust we prefer ownership clarity by default).
///
/// If `a.len() < b.len()`, the trailing entries copy `b`'s values
/// unchanged. If `a.len() > b.len()`, the trailing entries of `a` are
/// ignored (matching d3 — the result has `b.len()` elements).
pub fn interpolate_number_array(
    a: &[f64],
    b: &[f64],
) -> impl Fn(f64) -> Vec<f64> + use<> {
    // Eagerly clone so the closure outlives the input slices.
    let a: Vec<f64> = a.to_vec();
    let b: Vec<f64> = b.to_vec();
    let n = a.len().min(b.len());
    move |t| {
        let mut c = b.clone();
        for i in 0..n {
            c[i] = a[i] * (1.0 - t) + b[i] * t;
        }
        c
    }
}

/// Element-wise interpolation of arbitrary-typed slices using a
/// per-element interpolator factory.
///
/// `make` is called once per matched index with `(&a_i, &b_i)` and must
/// return a `Box<dyn Fn(f64) -> T>` for that pair. This is the closest
/// faithful port of d3's `genericArray` — but Rust's static typing means
/// `T` is fixed (use [`interpolate_number_array`] for `f64`, write an
/// equivalent helper for your own type).
///
/// Trailing entries of `b` (when `a.len() < b.len()`) are cloned through
/// constant.
pub fn interpolate_array_with<T, F>(
    a: &[T],
    b: &[T],
    mut make: F,
) -> impl Fn(f64) -> Vec<T> + use<T, F>
where
    T: Clone + 'static,
    F: FnMut(&T, &T) -> Box<dyn Fn(f64) -> T>,
{
    let n = a.len().min(b.len());
    let mut interps: Vec<Box<dyn Fn(f64) -> T>> = Vec::with_capacity(n);
    for i in 0..n {
        interps.push(make(&a[i], &b[i]));
    }
    let b_owned: Vec<T> = b.to_vec();
    move |t| {
        let mut out = b_owned.clone();
        for (i, x) in interps.iter().enumerate() {
            out[i] = x(t);
        }
        out
    }
}

/// Convenience wrapper specialised to nested numeric vectors. Uses
/// [`interpolate_number_array`] per inner row. Useful for matrix-shaped
/// data.
pub fn interpolate_number_matrix(
    a: &[Vec<f64>],
    b: &[Vec<f64>],
) -> impl Fn(f64) -> Vec<Vec<f64>> + use<> {
    interpolate_array_with(a, b, |a_row, b_row| {
        let f = interpolate_number_array(a_row, b_row);
        Box::new(f)
    })
}

/// Per-key linear interpolation over two string-keyed `f64` maps.
///
/// Equivalent to d3's `interpolateObject(a, b)` projected onto numeric
/// values. Keys present only in `b` are passed through constant; keys
/// present only in `a` are dropped (matching d3's behavior of iterating
/// `for (k in b)`).
///
/// For arbitrary-typed objects, see [`interpolate_object_with`].
pub fn interpolate_number_object(
    a: &HashMap<String, f64>,
    b: &HashMap<String, f64>,
) -> impl Fn(f64) -> HashMap<String, f64> + use<> {
    let a_owned = a.clone();
    let b_owned = b.clone();
    let mut shared_keys: Vec<String> = Vec::new();
    let mut constant_keys: Vec<String> = Vec::new();
    for (k, _) in b_owned.iter() {
        if a_owned.contains_key(k) {
            shared_keys.push(k.clone());
        } else {
            constant_keys.push(k.clone());
        }
    }
    move |t| {
        let mut out = HashMap::with_capacity(b_owned.len());
        for k in &shared_keys {
            let av = a_owned[k];
            let bv = b_owned[k];
            out.insert(k.clone(), av * (1.0 - t) + bv * t);
        }
        for k in &constant_keys {
            out.insert(k.clone(), b_owned[k]);
        }
        out
    }
}

/// Per-key interpolation over two `HashMap<String, T>`s using a
/// per-key interpolator factory. Mirrors d3's generic `interpolateObject`.
///
/// Keys present only in `b` are cloned through constant. Keys present
/// only in `a` are dropped.
pub fn interpolate_object_with<T, F>(
    a: &HashMap<String, T>,
    b: &HashMap<String, T>,
    mut make: F,
) -> impl Fn(f64) -> HashMap<String, T> + use<T, F>
where
    T: Clone + 'static,
    F: FnMut(&T, &T) -> Box<dyn Fn(f64) -> T>,
{
    type KeyedSlot<T> = (String, Box<dyn Fn(f64) -> T>);
    let mut shared: Vec<KeyedSlot<T>> = Vec::new();
    let mut constants: Vec<(String, T)> = Vec::new();
    for (k, bv) in b.iter() {
        if let Some(av) = a.get(k) {
            shared.push((k.clone(), make(av, bv)));
        } else {
            constants.push((k.clone(), bv.clone()));
        }
    }
    move |t| {
        let mut out = HashMap::with_capacity(shared.len() + constants.len());
        for (k, fx) in shared.iter() {
            out.insert(k.clone(), fx(t));
        }
        for (k, v) in constants.iter() {
            out.insert(k.clone(), v.clone());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vf(xs: &[f64]) -> Vec<f64> { xs.to_vec() }

    #[test]
    fn number_array_matches_d3_simple() {
        let i = interpolate_number_array(&[2.0, 12.0], &[4.0, 24.0]);
        assert_eq!(i(0.5), vec![3.0, 18.0]);
    }

    #[test]
    fn number_array_uses_b_length() {
        // a longer than b: extra a entries ignored.
        let i = interpolate_number_array(&[2.0, 12.0, 12.0], &[4.0, 24.0]);
        assert_eq!(i(0.5), vec![3.0, 18.0]);
        // b longer than a: extra b entries copied as constants.
        let i = interpolate_number_array(&[2.0, 12.0], &[4.0, 24.0, 12.0]);
        assert_eq!(i(0.5), vec![3.0, 18.0, 12.0]);
    }

    #[test]
    fn number_array_empty_a() {
        let i = interpolate_number_array(&[], &[2.0, 12.0]);
        assert_eq!(i(0.5), vec![2.0, 12.0]);
    }

    #[test]
    fn number_array_empty_b() {
        let i = interpolate_number_array(&[2.0, 12.0], &[]);
        assert_eq!(i(0.5), Vec::<f64>::new());
    }

    #[test]
    fn number_array_exact_endpoints() {
        let i = interpolate_number_array(&[2e42], &[355.0]);
        assert_eq!(i(0.0), vec![2e42]);
        assert_eq!(i(1.0), vec![355.0]);
    }

    #[test]
    fn number_matrix_interpolates_nested() {
        let a = vec![vec![2.0, 12.0]];
        let b = vec![vec![4.0, 24.0]];
        let i = interpolate_number_matrix(&a, &b);
        assert_eq!(i(0.5), vec![vec![3.0, 18.0]]);
    }

    #[test]
    fn number_object_basic() {
        let mut a = HashMap::new();
        a.insert("x".into(), 0.0);
        a.insert("y".into(), 100.0);
        let mut b = HashMap::new();
        b.insert("x".into(), 50.0);
        b.insert("y".into(), 0.0);
        let i = interpolate_number_object(&a, &b);
        let r = i(0.5);
        assert_eq!(r["x"], 25.0);
        assert_eq!(r["y"], 50.0);
    }

    #[test]
    fn number_object_uses_b_keys_only() {
        let mut a = HashMap::new();
        a.insert("x".into(), 0.0);
        a.insert("z".into(), 99.0); // only in a
        let mut b = HashMap::new();
        b.insert("x".into(), 100.0);
        b.insert("y".into(), 5.0); // only in b
        let i = interpolate_number_object(&a, &b);
        let r = i(0.5);
        // x interpolated, y constant from b, z dropped.
        assert_eq!(r["x"], 50.0);
        assert_eq!(r["y"], 5.0);
        assert!(!r.contains_key("z"));
    }

    #[test]
    fn array_with_custom_interpolator() {
        // Use generic helper to interpolate Vec<u8> as if it were f64.
        let a: Vec<f64> = vf(&[10.0, 20.0]);
        let b: Vec<f64> = vf(&[30.0, 40.0]);
        let i = interpolate_array_with(&a, &b, |&av, &bv| {
            Box::new(move |t: f64| av * (1.0 - t) + bv * t)
        });
        assert_eq!(i(0.5), vec![20.0, 30.0]);
    }
}
