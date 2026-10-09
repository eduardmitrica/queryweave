/// Uppercase the first letter of a string.
#[must_use]
pub fn uppercase_first_letter(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or_else(String::new, |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    })
}

/// Split identifier string into segments by Pascal case.
///
/// # Errors
///
/// Returns a type erased error under a [`Box`] (due to its size) if the splitting fails.
pub fn split_pascal_case(target: &str) -> Result<Vec<&str>, Box<dyn std::error::Error>> {
    let re = fancy_regex::Regex::new(r"[A-Z]+(?=[A-Z][a-z]|$)|[A-Z][a-z0-9]*")?;

    let mut parts = Vec::new();
    for mat in re.find_iter(target) {
        parts.push(mat?.into());
    }

    if parts.is_empty() || parts.join("") != target {
        return Err("Invalid PascalCase string".into());
    }

    Ok(parts)
}

/// Unwrap `Option<T>`, returning `Some(T)` if `ty` is `Option<T>`.
///
/// This is the one syntactic type inspection the derives do: whether a field
/// is optional (or, for merge patches, the unsupported `Option<Option<T>>`) is a matter of its
/// shape. Everything about the type itself is left to the compiler.
#[must_use]
pub fn unwrap_option(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(tp) = ty
        && let Some(seg) = tp.path.segments.last()
        && seg.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &seg.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }

    None
}

/// `tokens` with every token (nested ones included) spanned to `span`.
///
/// For generated code that must be reported at what the user wrote: with any
/// call-site token left in an expression, rustc reports the whole expression at
/// the derive instead. E.g. a runtime crate's path from the conventions,
/// interpolated into a check spanned to a field's type.
#[must_use]
pub fn respanned(
    tokens: &impl quote::ToTokens,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    fn respan(
        stream: proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        stream
            .into_iter()
            .map(|mut tree| {
                if let proc_macro2::TokenTree::Group(group) = &tree {
                    let mut respanned_group =
                        proc_macro2::Group::new(group.delimiter(), respan(group.stream(), span));
                    respanned_group.set_span(span);
                    tree = proc_macro2::TokenTree::Group(respanned_group);
                } else {
                    tree.set_span(span);
                }
                tree
            })
            .collect()
    }

    respan(tokens.to_token_stream(), span)
}

#[cfg(test)]
mod tests {
    use super::{respanned, split_pascal_case, unwrap_option, uppercase_first_letter};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    fn ty(source: &str) -> syn::Type {
        syn::parse_str::<syn::Type>(source).unwrap()
    }

    #[rstest]
    #[case("aVeryLongString", "AVeryLongString")]
    #[case("AVeryLongString", "AVeryLongString")]
    #[case("x", "X")]
    // A leading non-alphabetic character has no uppercase form and is kept.
    #[case("1abc", "1abc")]
    #[case("", "")]
    fn uppercase_first_letter_only_touches_the_first_character(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(uppercase_first_letter(input), expected);
    }

    #[rstest]
    #[case("AVeryLongString", vec!["A", "Very", "Long", "String"])]
    #[case("UserModel", vec!["User", "Model"])]
    #[case("Model", vec!["Model"])]
    // Runs of capitals belong to the segment that follows them.
    #[case("HTTPServer", vec!["HTTP", "Server"])]
    // Digits attach to the preceding segment.
    #[case("Order2Model", vec!["Order2", "Model"])]
    fn split_pascal_case_splits_on_segment_boundaries(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        assert_eq!(split_pascal_case(input).unwrap(), expected);
    }

    #[rstest]
    // Not pascal cased: leading lowercase, snake case, and the empty string all
    // fail rather than yielding a partial split.
    #[case("aVeryLongString")]
    #[case("a_very_long_string")]
    #[case("User_Model")]
    #[case("")]
    fn split_pascal_case_rejects_non_pascal_case(#[case] input: &str) {
        assert!(split_pascal_case(input).is_err());
    }

    #[test]
    fn unwrap_option_peels_exactly_one_layer() {
        assert_eq!(unwrap_option(&ty("Option<u32>")), Some(&ty("u32")));
        assert_eq!(
            unwrap_option(&ty("Option<Option<u32>>")),
            Some(&ty("Option<u32>"))
        );
    }

    #[rstest]
    // Anything that is not an `Option<T>` path yields `None`.
    #[case("u32")]
    #[case("Vec<u32>")]
    #[case("Option")]
    #[case("&str")]
    fn unwrap_option_returns_none_for_non_options(#[case] source: &str) {
        assert_eq!(unwrap_option(&ty(source)), None);
    }

    // Respanning keeps every token, inside groups too.
    #[test]
    fn respanned_keeps_the_tokens() {
        let tokens: syn::Type = syn::parse_quote!(::core::option::Option<(u8, [u16; 2])>);

        assert_eq!(
            respanned(&tokens, proc_macro2::Span::call_site()).to_string(),
            quote::ToTokens::to_token_stream(&tokens).to_string()
        );
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_uppercase_first_letter() {
        use crate::utils::uppercase_first_letter;
        use pretty_assertions::assert_eq;
        let uppercase_1 = uppercase_first_letter("aVeryLongString");
        let uppercase_2 = uppercase_first_letter("AVeryLongString");

        assert_eq!(uppercase_2, uppercase_1);
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_split_pascal_case() {
        use crate::utils::split_pascal_case;
        use pretty_assertions::assert_eq;
        let segments = split_pascal_case("AVeryLongString").unwrap();
        assert_eq!(vec!["A", "Very", "Long", "String"], segments);

        let segments_result = split_pascal_case("aVeryLongString");
        assert!(segments_result.is_err());

        let segments_result = split_pascal_case("a_very_long_string");
        assert!(segments_result.is_err());
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_unwrap_option() {
        use crate::utils::unwrap_option;
        use pretty_assertions::assert_eq;
        use syn::{Type, parse_str};

        let ty = parse_str::<Type>("Option<u32>").unwrap();
        let inner_ty_option = unwrap_option(&ty);

        let expected_inner_ty = parse_str::<Type>("u32").unwrap();
        let expected_inner_ty_option = Some(&expected_inner_ty);

        assert_eq!(expected_inner_ty_option, inner_ty_option);
    }

    // Formerly the item's doc example.
    #[test]
    fn doc_example_respanned() {
        use crate::utils::respanned;
        let path: syn::Path = syn::parse_quote!(::queryweave::RawValue);
        let span = proc_macro2::Span::call_site();

        assert_eq!(
            respanned(&path, span).to_string(),
            ":: queryweave :: RawValue"
        );
    }
}
