use queryweave::RawValue as _;
use queryweave_derive::RawValue;

#[derive(RawValue)]
struct Username<T = String>(T);

#[derive(RawValue)]
struct OrderId(i32);

fn main() {
    let _: String = Username(String::new()).to_raw();
    let _: i32 = OrderId(1).to_raw();
}
