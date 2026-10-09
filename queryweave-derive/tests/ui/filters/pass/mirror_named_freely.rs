use queryweave_derive::Filters;

// The mirror names the model's generated types through `queryweave::Filterable`,
// never by rebuilding them from its own name, so the two names need not share
// a stem or a suffix.
#[derive(Filters)]
pub struct CustomerRecord {
    #[filter(sw)]
    #[sortable]
    pub name: String,
}

#[derive(Filters)]
#[filters(mirrors = CustomerRecord)]
pub struct AccountDto {
    #[filter(sw)]
    #[sortable]
    pub name: String,
}

fn main() {
    let query = CustomersQueryRecord::from(AccountsQueryParams::default());
    let _ = query.and.name_sw;
}
