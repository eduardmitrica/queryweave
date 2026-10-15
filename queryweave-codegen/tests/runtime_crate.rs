//! The runtime crate every derive's generated code names: the default of its
//! layer, or the one given by `#[queryweave(crate = path)]`.

use queryweave_codegen::conventions::DefaultConventions;

type Derive = fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream;

/// Each derive with an input it accepts (`#attrs` stands for the attributes
/// under test) and the runtime crate its layer names by default.
fn cases() -> Vec<(&'static str, Derive, proc_macro2::TokenStream, &'static str)> {
    vec![
        (
            "Filters (root)",
            queryweave_codegen::filters::<DefaultConventions>,
            quote::quote! { struct OrderModel { #[filter(eq)] #[sortable] value: i32 } },
            "queryweave",
        ),
        (
            "Filters (plain mirror)",
            queryweave_codegen::filters::<DefaultConventions>,
            quote::quote! {
                #[filters(mirrors = OrderModel)]
                struct OrderDto { #[filter(eq)] #[sortable] value: i32 }
            },
            "queryweave",
        ),
        (
            "Filters (API mirror)",
            queryweave_codegen::filters::<DefaultConventions>,
            quote::quote! {
                #[filters(mirrors = OrderModel, serde, utoipa, validator)]
                struct OrderDto { #[filter(eq)] #[sortable] value: i32 }
            },
            "queryweave_api",
        ),
        (
            "MergePatch",
            queryweave_codegen::merge_patch::<DefaultConventions>,
            quote::quote! {
                #[merge_patch(patch = MergePatchOrderModel)]
                struct OrderModel { value: i32 }
            },
            "queryweave",
        ),
        (
            "MergePatchNullableFields",
            queryweave_codegen::merge_patch_nullable_fields::<DefaultConventions>,
            quote::quote! { struct MergePatchOrderModel { note: Patch<String> } },
            "queryweave",
        ),
        (
            "RawValue",
            queryweave_codegen::raw_value::<DefaultConventions>,
            quote::quote! { struct Label<T = String>(T); },
            "queryweave",
        ),
        (
            "ValidateNonNullable",
            queryweave_codegen::validate_non_nullable::<DefaultConventions>,
            quote::quote! {
                #[validate_non_nullable(mirrors = MergePatchOrderModel, validator)]
                struct MergePatchOrderDto { note: Patch<String> }
            },
            // Only core items (`NullableFields`, `PatchKind`): the core.
            "queryweave",
        ),
        (
            "AppendToQuery",
            queryweave_codegen::append_to_query::<DefaultConventions>,
            quote::quote! {
                #[append_to_query(columns = Order, filters = OrdersFilterOptionsModelWrapper)]
                struct OrdersQueryModelWrapper(OrdersQueryModel);
            },
            "queryweave_sql",
        ),
        (
            "AppendConditions",
            queryweave_codegen::append_conditions::<DefaultConventions>,
            quote::quote! {
                #[append_conditions(columns = Order)]
                struct OrdersFilterOptionsModelWrapper(OrdersFilterOptionsModel);
            },
            "queryweave_sql",
        ),
        (
            "Columns",
            queryweave_codegen::columns::<DefaultConventions>,
            quote::quote! {
                #[columns(model = OrderModel)]
                struct Order { value: i32 }
            },
            "queryweave_sql",
        ),
    ]
}

/// `item` with `attribute` added to its struct attributes.
fn with_attribute(
    item: &proc_macro2::TokenStream,
    attribute: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mut item: syn::DeriveInput = syn::parse_quote!(#item);
    item.attrs.push(syn::parse_quote!(#[#attribute]));
    quote::ToTokens::into_token_stream(item)
}

/// The runtime crates the generated code names, without the leading `::`.
fn runtime_crates_named(code: &str) -> Vec<&'static str> {
    ["queryweave_api", "queryweave_sql", "queryweave"]
        .into_iter()
        .filter(|name| {
            // `queryweave` alone, not as the prefix of the layer crates.
            code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|word| word == *name)
        })
        .collect()
}

#[test]
fn test_each_derive_names_the_runtime_crate_of_its_layer() {
    for (derive, expand, item, runtime) in cases() {
        let code = expand(item).to_string();

        assert!(!code.contains("compile_error"), "{derive}: {code}");
        assert_eq!(
            runtime_crates_named(&code),
            vec![runtime],
            "{derive}: {code}"
        );
        assert!(
            code.contains(&format!(":: {runtime} ::")),
            "{derive}: {code}"
        );
    }
}

#[test]
fn test_the_crate_option_overrides_the_runtime_crate() {
    for (derive, expand, item, _) in cases() {
        let code = expand(with_attribute(
            &item,
            &quote::quote!(queryweave(crate = my::runtime)),
        ))
        .to_string();

        assert!(!code.contains("compile_error"), "{derive}: {code}");
        assert_eq!(
            runtime_crates_named(&code),
            Vec::<&str>::new(),
            "{derive}: {code}"
        );
        assert!(code.contains("my :: runtime ::"), "{derive}: {code}");
    }
}

#[test]
fn test_a_malformed_crate_option_is_an_error() {
    for (derive, expand, item, _) in cases() {
        for attribute in [
            quote::quote!(queryweave(krate = my::runtime)),
            quote::quote!(queryweave(crate)),
            quote::quote!(queryweave(crate = "my::runtime")),
        ] {
            let code = expand(with_attribute(&item, &attribute)).to_string();

            assert!(
                code.contains("compile_error"),
                "{derive} with #[{attribute}]: {code}"
            );
        }
    }
}

/// The crates a mirror's code names, of those each integration brings in.
fn integration_crates_named(code: &str) -> Vec<&'static str> {
    [
        "serde",
        "utoipa",
        "validator",
        "schemars",
        "garde",
        "queryweave_api",
    ]
    .into_iter()
    .filter(|name| {
        code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .any(|word| word == *name)
    })
    .collect()
}

// A crate deriving a mirror depends only on the crates behind the flags it
// writes: a plain mirror names the core runtime alone.
#[test]
fn test_a_mirror_names_only_the_crates_of_its_integrations() {
    for (flags, crates) in [
        (quote::quote!(), vec![]),
        (quote::quote!(, serde), vec!["serde", "queryweave_api"]),
        (quote::quote!(, utoipa), vec!["utoipa", "queryweave_api"]),
        (
            quote::quote!(, schemars),
            vec!["schemars", "queryweave_api"],
        ),
        // garde needs no serializer: its mirror names the core runtime.
        (quote::quote!(, garde), vec!["garde"]),
        (
            quote::quote!(, serde, validator),
            vec!["serde", "validator", "queryweave_api"],
        ),
        (
            quote::quote!(, serde, utoipa, validator),
            vec!["serde", "utoipa", "validator", "queryweave_api"],
        ),
    ] {
        let code = queryweave_codegen::filters::<DefaultConventions>(quote::quote! {
            #[filters(mirrors = OrderModel #flags)]
            struct OrderDto { #[filter(eq)] #[sortable] value: i32 }
        })
        .to_string();

        assert!(!code.contains("compile_error"), "{flags}: {code}");
        assert_eq!(integration_crates_named(&code), crates, "{flags}: {code}");
    }
}
