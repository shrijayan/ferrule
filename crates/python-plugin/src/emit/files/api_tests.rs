//! `tests/api_resources/**`. Port of `python-plugin/src/emit/files/api-tests.ts`.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::model::SdkType;

/// Helpers a test module imports from `<pkg>._utils` because an example value uses them:
/// `parse_date` and `parse_datetime`.
pub type Uses = BTreeSet<&'static str>;

/// A Python literal for an example of `ty`, laid out the way ruff formats the oracle tests.
///
/// # Panics
///
/// Until the port lands (ADR 0014).
#[must_use]
pub fn example_value(
    ty: &SdkType,
    uses: &mut Uses,
    indent: &str,
    example: Option<&Value>,
    fallback: &str,
    bound: Option<f64>,
) -> String {
    let _ = (ty, uses, indent, example, fallback, bound);
    todo!("port of exampleValue")
}
