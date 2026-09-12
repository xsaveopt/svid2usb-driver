fn main() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-undefined,dynamic_lookup");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-no_fixup_chains");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/svid2usb");
}
