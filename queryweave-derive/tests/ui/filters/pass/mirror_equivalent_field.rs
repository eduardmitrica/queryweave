use queryweave_derive::Filters;

#[derive(Filters)]
pub struct InvoiceModel {
    #[filter(eq, sw)]
    #[sortable]
    pub number: String,

    #[filter(lt)]
    pub total: f64,
}

// Renamed and same-named fields side by side.
#[derive(Filters)]
#[filters(mirrors = InvoiceModel)]
pub struct InvoiceDto {
    #[filter(eq, sw)]
    #[sortable]
    #[equivalent_field(number)]
    pub invoice_number: String,

    #[filter(lt)]
    pub total: f64,
}

fn main() {
    let filters = InvoicesFilterOptionsModel::from(InvoicesQueryParams::default());
    let _ = (filters.number_eq, filters.number_sw, filters.total_lt);
}
