#[rustversion::nightly]
fn nightly() {
    println!("cargo:rustc-cfg=pme_nightly");
}

#[rustversion::not(nightly)]
fn nightly() {}

fn main() {
    nightly()
}
