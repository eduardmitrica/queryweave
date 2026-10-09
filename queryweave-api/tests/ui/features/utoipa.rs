// What the code generated for a mirror with the `utoipa` flag checks: builds
// with the `utoipa` feature, and names the feature to enable without it.
const fn requires_feature<T: queryweave_api::features::Utoipa>() {}

const _: () = requires_feature::<queryweave_api::SortDirectionDto>();

fn main() {}
