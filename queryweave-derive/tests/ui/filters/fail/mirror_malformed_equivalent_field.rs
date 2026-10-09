use queryweave_derive::Filters;

#[derive(Filters)]
pub struct InvoiceModel {
    #[filter(eq)]
    pub number: String,
}

#[derive(Filters)]
#[filters(mirrors = InvoiceModel)]
pub struct InvoiceDto {
    #[filter(eq)]
    #[equivalent_field(number, client)]
    pub invoice_number: String,
}

fn main() {}
