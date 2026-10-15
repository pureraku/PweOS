use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let assets = root.join("assets");
    println!("cargo:rerun-if-changed={}", assets.display());

    let mut images: Vec<_> = fs::read_dir(&assets)
        .expect("assets directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rgb332"))
        .collect();
    images.sort();

    let mut generated = String::from("pub static BUNDLED_IMAGES: &[(&str, &[u8])] = &[\n");
    for image in images {
        println!("cargo:rerun-if-changed={}", image.display());
        let Some(name) = image.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        assert!(name.len() < 32, "image filename is too long for RAMFS: {name}");
        assert_eq!(
            fs::metadata(&image).unwrap().len(),
            320 * 200,
            "RGB332 image must contain exactly 320x200 pixels: {}",
            image.display()
        );
        let image = image.canonicalize().unwrap();
        generated.push_str(&format!(
            "    ({:?}, include_bytes!({:?})),\n",
            name,
            image.to_string_lossy()
        ));
    }
    generated.push_str("];\n");

    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bundled_images.rs");
    fs::write(output, generated).unwrap();
}
