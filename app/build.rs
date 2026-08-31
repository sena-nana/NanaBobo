fn main() {
    nana_app_icon::embed_windows_from("icons/icon.ico");
    println!("cargo:rerun-if-changed=../ui/dist");
}
