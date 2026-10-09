use proc_macro2::Span;
use quote::ToTokens;

/// Wraps a message with a helper hint with a [`syn::Error`].
///
/// # Errors
///
/// Always returns a [`syn::Error`].
pub fn abort<T>(span: Span, msg: &str, help: &str) -> syn::Result<T> {
    Err(syn::Error::new(span, message(msg, help)))
}

/// Wrapper over [`abort`] with call site hygiene.
///
/// # Errors
///
/// Always returns a [`syn::Error`].
pub fn abort_call_site<T>(msg: &str, help: &str) -> syn::Result<T> {
    abort(Span::call_site(), msg, help)
}

/// Span the given syntax tree over a [`syn::Error`].
///
/// # Errors
///
/// Always returns a [`syn::Error`].
pub fn abort_spanned<T>(tokens: impl ToTokens, msg: &str, help: &str) -> syn::Result<T> {
    Err(syn::Error::new_spanned(tokens, message(msg, help)))
}

/// Properly format a the error message (msg) and the suggested fix (help)
#[must_use]
pub fn message(msg: &str, help: &str) -> String {
    format!("{msg}\n\n= help: {help}\n\n")
}

/// Both values, or the errors of every one that failed (in order).
///
/// # Errors
///
/// Returns the combined errors of `first` and `second`.
pub fn both<A, B>(first: syn::Result<A>, second: syn::Result<B>) -> syn::Result<(A, B)> {
    match (first, second) {
        (Ok(first), Ok(second)) => Ok((first, second)),
        (Err(mut error), Err(other)) => {
            error.combine(other);
            Err(error)
        },
        (Err(error), Ok(_)) | (Ok(_), Err(error)) => Err(error),
    }
}

/// Errors collected while going through a derive's input (e.g. one per
/// malformed field), so all of them are reported rather than only the first.
#[derive(Default)]
pub struct Errors {
    combined: Option<syn::Error>,
}

impl Errors {
    /// Collect an error.
    pub fn push(&mut self, error: syn::Error) {
        match &mut self.combined {
            Some(combined) => combined.combine(error),
            None => self.combined = Some(error),
        }
    }

    /// Collect the error of `result`, if any, returning its value otherwise.
    pub fn check<T>(&mut self, result: syn::Result<T>) -> Option<T> {
        result.map_err(|error| self.push(error)).ok()
    }

    /// `value`, or every collected error.
    ///
    /// # Errors
    ///
    /// Returns the collected errors, combined, if there are any.
    pub fn finish<T>(self, value: T) -> syn::Result<T> {
        self.combined.map_or(Ok(value), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::{Errors, abort, abort_call_site, abort_spanned, both, message};
    use pretty_assertions::assert_eq;

    fn failure(text: &str) -> syn::Error {
        syn::Error::new(proc_macro2::Span::call_site(), text)
    }

    fn messages(error: syn::Error) -> Vec<String> {
        error.into_iter().map(|error| error.to_string()).collect()
    }

    #[test]
    fn both_keeps_both_values_or_every_error_in_order() {
        assert_eq!(both::<_, u8>(Ok(1), Ok(2)).unwrap(), (1, 2));
        assert_eq!(
            messages(both::<u8, u8>(Err(failure("first")), Err(failure("second"))).unwrap_err()),
            vec!["first", "second"]
        );
        assert_eq!(
            messages(both::<u8, u8>(Ok(1), Err(failure("second"))).unwrap_err()),
            vec!["second"]
        );
    }

    #[test]
    fn errors_finish_with_the_value_or_every_collected_error() {
        assert_eq!(Errors::default().finish(7).unwrap(), 7);

        let mut errors = Errors::default();
        errors.push(failure("first"));
        assert_eq!(errors.check::<u8>(Err(failure("second"))), None);
        assert_eq!(errors.check::<u8>(Ok(3)), Some(3));

        assert_eq!(
            messages(errors.finish(7).unwrap_err()),
            vec!["first", "second"]
        );
    }

    // The exact layout is what the `trybuild` `.stderr` fixtures are matched
    // against, so it is pinned here too: the UI tests are excluded from the
    // profile the mutation run uses.
    #[test]
    fn message_lays_out_the_error_and_the_help_hint() {
        assert_eq!(
            message("Something went wrong", "Try this instead"),
            "Something went wrong\n\n= help: Try this instead\n\n"
        );
    }

    #[test]
    fn message_keeps_both_halves_distinct() {
        let formatted = message("the-message", "the-help");

        assert!(formatted.starts_with("the-message"));
        assert!(formatted.contains("= help: the-help"));
    }

    #[test]
    fn abort_returns_an_error_carrying_the_formatted_message() {
        let error = abort::<()>(proc_macro2::Span::call_site(), "msg", "help").unwrap_err();

        assert_eq!(error.to_string(), message("msg", "help"));
    }

    #[test]
    fn abort_call_site_returns_an_error_carrying_the_formatted_message() {
        let error = abort_call_site::<()>("msg", "help").unwrap_err();

        assert_eq!(error.to_string(), message("msg", "help"));
    }

    #[test]
    fn abort_spanned_returns_an_error_carrying_the_formatted_message() {
        let tokens = quote::quote! { struct UserModel; };
        let error = abort_spanned::<()>(tokens, "msg", "help").unwrap_err();

        assert_eq!(error.to_string(), message("msg", "help"));
    }
}
