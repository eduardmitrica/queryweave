// What the code generated for a mirror with the `serde` flag checks: builds
// with the `serde` feature, and names the feature to enable without it.
const fn requires_feature<T: queryweave_api::features::Serde>() {}

const _: () = requires_feature::<queryweave_api::SortDirectionDto>();

fn main() {}
