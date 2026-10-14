//! The SQL every query builder shares: operators, directions, `LIKE`
//! patterns and page offsets, portable across Postgres, MySQL/MariaDB and
//! SQLite.
//!
//! Only standard SQL is written: comparisons, `LIKE … ESCAPE`, `ORDER BY`,
//! `col = value` assignments and `LIMIT … OFFSET …`, every value bound, never
//! interpolated. The targets (`backend`) write it with these helpers, which
//! are public for a target of your own.

use crate::PageTarget;
use queryweave::{FilterOperator, SortDirectionModel};

/// The character escaping the `LIKE` metacharacters in a bound pattern
/// ([`escape_like`]), to name in the clause: `… LIKE ? ESCAPE '!'`.
///
/// Not the backslash: MySQL reads `'\'` as an escape inside string literals
/// (so `ESCAPE '\'` does not even parse), and SQLite has no default escape
/// character at all. `!` means the same everywhere.
pub const LIKE_ESCAPE: char = '!';

/// The SQL comparison operator for a filter operator.
#[must_use]
pub const fn sql_operator(operator: FilterOperator) -> &'static str {
    match operator {
        FilterOperator::LessThan => "<",
        FilterOperator::Equal => "=",
        FilterOperator::GreaterThan => ">",
        FilterOperator::StartsWith | FilterOperator::Contains | FilterOperator::EndsWith => "LIKE",
    }
}

/// The SQL keyword for a sort direction.
#[must_use]
pub const fn sql_direction(direction: SortDirectionModel) -> &'static str {
    match direction {
        SortDirectionModel::Ascending => "ASC",
        SortDirectionModel::Descending => "DESC",
    }
}

/// The `LIKE` pattern to bind for a `LIKE` operator: the escaped text wrapped
/// in `%` wildcards. `None` for the comparison operators, which bind the value
/// itself.
///
/// Kept separate so the wildcard placement can be unit-tested directly: the
/// value is pushed as a bound parameter and therefore never appears in the
/// rendered SQL.
#[must_use]
pub fn like_pattern(operator: FilterOperator, text: &str) -> Option<String> {
    let text = escape_like(text);

    match operator {
        FilterOperator::StartsWith => Some(format!("{text}%")),
        FilterOperator::Contains => Some(format!("%{text}%")),
        FilterOperator::EndsWith => Some(format!("%{text}")),
        FilterOperator::LessThan | FilterOperator::Equal | FilterOperator::GreaterThan => None,
    }
}

/// Escape the `LIKE` metacharacters (`%`, `_` and [`LIKE_ESCAPE`] itself) so
/// user input matches literally instead of acting as wildcards.
#[must_use]
pub fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        if matches!(character, '%' | '_') || character == LIKE_ESCAPE {
            escaped.push(LIKE_ESCAPE);
        }
        escaped.push(character);
    }

    escaped
}

/// The row offset of page `page` (0 based) of `page_size` rows, saturated at
/// `i64::MAX`: `u32 * u32` always fits a `u64`, and an offset past every
/// database's signed 64-bit row count just yields an empty page.
#[must_use]
pub fn page_offset(page: u32, page_size: u32) -> i64 {
    i64::try_from(u64::from(page) * u64::from(page_size)).unwrap_or(i64::MAX)
}

/// Write a `LIMIT page_size OFFSET page * page_size` clause, binding both,
/// when both a page and a page size are requested; nothing otherwise.
///
/// See [`PageTarget::write_page`].
pub fn push_page<Q: PageTarget + ?Sized>(query: &mut Q, page: Option<u32>, page_size: Option<u32>) {
    query.write_page(page, page_size);
}

#[cfg(test)]
mod tests {
    use super::{escape_like, like_pattern, page_offset};
    use pretty_assertions::assert_eq;
    use queryweave::FilterOperator;
    use rstest::rstest;

    // The `LIKE` operators bind the value wrapped in `%`; the comparisons bind
    // the typed value itself, so they have no pattern. The pattern is a bound
    // parameter and never appears in the rendered SQL, so it is asserted
    // through the extracted helper.
    #[rstest]
    #[case(FilterOperator::StartsWith, Some("needle%"))]
    #[case(FilterOperator::Contains, Some("%needle%"))]
    #[case(FilterOperator::EndsWith, Some("%needle"))]
    #[case(FilterOperator::Equal, None)]
    #[case(FilterOperator::LessThan, None)]
    #[case(FilterOperator::GreaterThan, None)]
    fn the_like_pattern_matches_the_operator(
        #[case] operator: FilterOperator,
        #[case] expected_pattern: Option<&str>,
    ) {
        assert_eq!(
            like_pattern(operator, "needle").as_deref(),
            expected_pattern
        );
    }

    // `LIKE` metacharacters (and the escape character itself) are escaped
    // with `!`, so they match literally; a backslash is an ordinary character.
    #[rstest]
    #[case("%", "!%")]
    #[case("_", "!_")]
    #[case("!", "!!")]
    #[case(r"\", r"\")]
    #[case(r"50%_off!\now", r"50!%!_off!!\now")]
    #[case("plain", "plain")]
    fn metacharacters_are_escaped_with_the_escape_character(
        #[case] value: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(escape_like(value), expected);
    }

    // Only the wildcards the operator adds stay live.
    #[test]
    fn the_pattern_wraps_the_escaped_text() {
        assert_eq!(
            like_pattern(FilterOperator::Contains, "50%").as_deref(),
            Some("%50!%%")
        );
    }

    // The offset is `page * page_size`, saturated rather than overflowing.
    #[rstest]
    #[case(0, 10, 0)]
    #[case(3, 10, 30)]
    #[case(u32::MAX, 1, 4_294_967_295)]
    #[case(u32::MAX, u32::MAX, i64::MAX)]
    fn the_offset_saturates(#[case] page: u32, #[case] page_size: u32, #[case] expected: i64) {
        assert_eq!(page_offset(page, page_size), expected);
    }
}
