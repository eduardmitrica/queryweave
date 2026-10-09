use queryweave_derive::Filters;

// A root model maps onto no other type, so it cannot rename a field onto one;
// the error points at the attribute.
#[derive(Filters)]
pub struct InvoiceModel {
    #[filter(eq)]
    #[equivalent_field(number)]
    pub invoice_number: String,
}

fn main() {}
