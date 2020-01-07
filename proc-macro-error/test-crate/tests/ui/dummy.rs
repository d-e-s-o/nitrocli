extern crate test_crate;
use test_crate::make_fn;

enum NeedDefault {
    A,
    B
}

make_fn!(need_default);

fn main() {
    let _ = NeedDefault::default();
}



