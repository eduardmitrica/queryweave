// What the code generated for a mirror with the `schemars` flag checks: builds
// with the `schemars` feature, and names the feature to enable without it.
const fn requires_feature<T: queryweave_api::features::Schemars>() {}

const _: () = requires_feature::<queryweave_api::SortDirectionDto>();

fn main() {}
