use queryweave_derive::Filters;

#[derive(Filters)]
pub struct InvoiceModel {
    #[filter(eq)]
    #[sortable]
    pub number: String,
}

// `numbr` is not a field of `InvoiceModel`: both the filter and the sort
// variant it maps onto are missing, reported at the name.
#[derive(Filters)]
#[filters(mirrors = InvoiceModel)]
pub struct InvoiceDto {
    #[filter(eq)]
    #[sortable]
    #[equivalent_field(numbr)]
    pub invoice_number: String,
}

fn main() {}
