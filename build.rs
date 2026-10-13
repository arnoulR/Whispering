fn main() {
    println!("cargo:rerun-if-changed=assets/whispering.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set("ProductName", "Whispering")
            .set("FileDescription", "Whispering — local dictation");
        if std::path::Path::new("assets/whispering.ico").exists() {
            resource.set_icon("assets/whispering.ico");
        }
        resource.compile().expect("Windows resources");
    }
}
