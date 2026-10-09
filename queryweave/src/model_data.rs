//! Data describing a model's fields, tagged with the model it came from.
//!
//! The derives hand the persistence layer filters, sorting and writes as
//! [`Conditions`], [`SortKeys`] and [`FieldValues`] of the generated type that
//! produced them, so the persistence layer takes the model from the data's own
//! type: it cannot check one model's columns and write another model's data.

use crate::{FieldValue, FilterCondition, PersistedFields, SortKey};
use std::marker::PhantomData;

/// Items (`T`) naming fields of the model `M`, e.g. the set filters of a
/// generated filter-options model.
///
/// Every item names one of `M`'s [`PersistedFields::FIELD_NAMES`]; that is
/// what lets the persistence layer check, from `M` alone, that each field it
/// will write has a column.
pub struct ModelData<M, T> {
    items: Vec<T>,
    // `fn() -> M`: the data does not own an `M`, so it is `Send`, `Sync` and
    // `'static` whatever `M` is.
    model: PhantomData<fn() -> M>,
}

/// The set filters of the model `M`, as database-agnostic conditions.
pub type Conditions<M> = ModelData<M, FilterCondition>;

/// The requested sorting of the model `M`, in order.
pub type SortKeys<M> = ModelData<M, SortKey>;

/// Fields of the model `M` to write, with their values.
pub type FieldValues<M> = ModelData<M, FieldValue>;

/// An item naming a domain field.
pub trait NamesField {
    /// The domain field named.
    fn field(&self) -> &'static str;
}

impl NamesField for FilterCondition {
    fn field(&self) -> &'static str {
        self.field
    }
}

impl NamesField for SortKey {
    fn field(&self) -> &'static str {
        self.field
    }
}

impl NamesField for FieldValue {
    fn field(&self) -> &'static str {
        self.field
    }
}

/// Tag `items` as data of the model `M`.
///
/// Not public API (re-exported as `queryweave::__private::model_data`): the
/// derives build the data of the types they generate, whose items name `M`'s
/// fields by construction. Built by hand (tests, a type implementing
/// [`PersistedFields`] of your own), each item must name one of `M`'s fields;
/// that is checked in debug builds.
#[must_use]
pub fn model_data<M: PersistedFields, T: NamesField>(items: Vec<T>) -> ModelData<M, T> {
    ModelData::new(items)
}

impl<M: PersistedFields, T: NamesField> ModelData<M, T> {
    /// Tag `items` as data of the model `M`; see [`model_data`].
    fn new(items: Vec<T>) -> Self {
        debug_assert!(
            items
                .iter()
                .all(|item| M::FIELD_NAMES.contains(&item.field())),
            "an item names a field that is not in the model's `PersistedFields::FIELD_NAMES`"
        );

        Self {
            items,
            model: PhantomData,
        }
    }
}

impl<M, T> ModelData<M, T> {
    /// Whether there are no items.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items, in order.
    #[must_use]
    pub const fn as_slice(&self) -> &[T] {
        self.items.as_slice()
    }

    /// The items, in order, untagged.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.items
    }
}

impl<M, T> IntoIterator for ModelData<M, T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

// Implemented by hand: derives would require `M` itself to implement them.

impl<M, T: std::fmt::Debug> std::fmt::Debug for ModelData<M, T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().entries(&self.items).finish()
    }
}

impl<M, T: Clone> Clone for ModelData<M, T> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            model: PhantomData,
        }
    }
}

impl<M, T: PartialEq> PartialEq for ModelData<M, T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

#[cfg(test)]
mod tests {
    use super::{Conditions, FieldValues, SortKeys};
    use crate::{
        FieldValue, FilterCondition, FilterOperator, PersistedFields, SortDirectionModel, SortKey,
        Value,
    };

    struct Order;

    impl PersistedFields for Order {
        const FIELD_NAMES: &'static [&'static str] = &["number", "value"];
    }

    fn sort_key(field: &'static str) -> SortKey {
        SortKey {
            field,
            direction: SortDirectionModel::Ascending,
        }
    }

    #[test]
    fn the_items_are_kept_in_order() {
        let sort_keys = SortKeys::<Order>::new(vec![sort_key("value"), sort_key("number")]);

        assert!(!sort_keys.is_empty());
        assert_eq!(
            sort_keys.as_slice(),
            &[sort_key("value"), sort_key("number")]
        );
        assert_eq!(
            sort_keys
                .into_iter()
                .map(|key| key.field)
                .collect::<Vec<_>>(),
            vec!["value", "number"]
        );
    }

    #[test]
    fn no_items_is_empty() {
        assert!(FieldValues::<Order>::new(Vec::new()).is_empty());
    }

    fn field_value(field: &'static str) -> FieldValue {
        FieldValue {
            field,
            value: Value::I32(Some(1)),
        }
    }

    // Items naming the model's fields are accepted, whatever their kind (the
    // check reads each kind's field).
    #[test]
    fn items_naming_the_models_fields_are_accepted() {
        let conditions = Conditions::<Order>::new(vec![FilterCondition {
            field: "number",
            operator: FilterOperator::Equal,
            value: Value::Text(None),
        }]);
        let field_values = FieldValues::<Order>::new(vec![field_value("value")]);
        let sort_keys = SortKeys::<Order>::new(vec![sort_key("number")]);

        assert_eq!(conditions.as_slice().len(), 1);
        assert_eq!(field_values.as_slice().len(), 1);
        assert_eq!(sort_keys.as_slice().len(), 1);
    }

    #[test]
    fn into_vec_returns_the_items_untagged() {
        assert_eq!(
            FieldValues::<Order>::new(vec![field_value("number"), field_value("value")]).into_vec(),
            vec![field_value("number"), field_value("value")]
        );
    }

    // Equality compares the items.
    #[test]
    fn data_is_equal_exactly_when_its_items_are() {
        let numbers = || FieldValues::<Order>::new(vec![field_value("number")]);

        assert_eq!(numbers(), numbers());
        assert_ne!(
            numbers(),
            FieldValues::<Order>::new(vec![field_value("value")])
        );
    }

    // The data debugs as the list of its items (without requiring the model
    // to be `Debug`).
    #[test]
    fn data_debugs_as_its_items() {
        let field_values = FieldValues::<Order>::new(vec![field_value("number")]);

        assert_eq!(
            format!("{field_values:?}"),
            format!("{:?}", vec![field_value("number")])
        );
    }

    // Debug builds reject an item naming a field the model does not list.
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "not in the model's")]
    fn an_item_naming_another_field_is_rejected() {
        let _ = FieldValues::<Order>::new(vec![FieldValue {
            field: "client",
            value: Value::Text(None),
        }]);
    }
}
