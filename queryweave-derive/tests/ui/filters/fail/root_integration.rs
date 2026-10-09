use queryweave_derive::Filters;

// The integrations are for a mirror's generated types; each one written on a
// root model is reported at its name.
#[derive(Filters)]
#[filters(serde, validator)]
pub struct InvoiceModel {
    #[filter(eq)]
    pub invoice_number: String,
}

fn main() {}
