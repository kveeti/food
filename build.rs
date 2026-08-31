fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("tailwind.css")
        .cwd("src")
        .render()
        .unwrap();
}
