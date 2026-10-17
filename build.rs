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
        let Some(name) = image.file_name().and_then(|name| name.to_str()) else { continue };
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
    generated.push_str("];\n\npub static VIDEO_FILES: &[(&str, u32, u32)] = &[\n");
    let mut videos: Vec<_> = fs::read_dir(&assets)
        .expect("assets directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "pwevid"))
        .collect();
    videos.sort();
    assert!(videos.len() <= 12, "the video disk index supports at most 12 videos");
    let mut next_sector = 1u32; // Sector zero is the video's index table.
    for video in videos {
        println!("cargo:rerun-if-changed={}", video.display());
        let Some(name) = video.file_name().and_then(|name| name.to_str()) else { continue };
        assert!(name.len() < 32, "video filename is too long for RAMFS: {name}");
        let bytes = fs::read(&video).unwrap();
        assert!(bytes.starts_with(b"PWEV"), "invalid PweOS video: {}", video.display());
        let sectors = bytes.len().div_ceil(512) as u32;
        generated.push_str(&format!("    ({:?}, {}, {}),\n", name, next_sector, bytes.len()));
        next_sector += sectors;
    }
    generated.push_str("];\n");

    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bundled_images.rs");
    fs::write(output, generated).unwrap();

    let programs_output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bundled_programs.rs");
    let mut programs = String::from("pub static BUNDLED_PROGRAMS: &[(&str, &[u8])] = &[\n");
    for name in ["hello.elf", "tcc.elf"] {
        let program = root.join("build").join(name);
        println!("cargo:rerun-if-changed={}", program.display());
        if program.exists() {
            let program = program.canonicalize().unwrap();
            programs.push_str(&format!(
                "    ({:?}, include_bytes!({:?})),\n",
                name,
                program.to_string_lossy()
            ));
        }
    }
    programs.push_str("];\n");
    fs::write(programs_output, programs).unwrap();

    let tcc_include = root.join("third_party/tinycc/include");
    println!("cargo:rerun-if-changed={}", tcc_include.display());
    let mut tcc_headers: Vec<_> = fs::read_dir(&tcc_include)
        .expect("TinyCC include directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    tcc_headers.sort();

    let tcc_runtime = root.join("build/libtcc1.a");
    println!("cargo:rerun-if-changed={}", tcc_runtime.display());
    let mut tcc_files = String::from("pub static BUNDLED_TCC_FILES: &[(&str, &[u8])] = &[\n");
    for header in tcc_headers {
        let Some(name) = header.file_name().and_then(|name| name.to_str()) else { continue };
        assert!(name.len() < 32, "TinyCC header filename is too long for RAMFS: {name}");
        let header = header.canonicalize().unwrap();
        tcc_files.push_str(&format!(
            "    ({:?}, include_bytes!({:?})),\n",
            format!("tcc/include/{name}"),
            header.to_string_lossy()
        ));
    }

    let pweos_include = root.join("examples");
    println!("cargo:rerun-if-changed={}", pweos_include.display());
    let mut pweos_headers = Vec::new();
    collect_headers(&pweos_include, &mut pweos_headers);
    pweos_headers.sort();
    for header in pweos_headers {
        let relative = header.strip_prefix(&pweos_include).unwrap();
        let install_path = format!("tcc/include/{}", relative.to_string_lossy());
        assert!(
            install_path.split('/').all(|part| part.len() < 32),
            "PweOS header path is too long for RAMFS: {install_path}"
        );
        let header = header.canonicalize().unwrap();
        tcc_files.push_str(&format!(
            "    ({:?}, include_bytes!({:?})),\n",
            install_path,
            header.to_string_lossy()
        ));
    }
    if tcc_runtime.exists() {
        let runtime = tcc_runtime.canonicalize().unwrap();
        tcc_files.push_str(&format!(
            "    (\"tcc/lib/libtcc1.a\", include_bytes!({:?})),\n",
            runtime.to_string_lossy()
        ));
    }
    tcc_files.push_str("];\n");
    let tcc_output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bundled_tcc.rs");
    fs::write(tcc_output, tcc_files).unwrap();
}

fn collect_headers(directory: &std::path::Path, headers: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory)
        .expect("PweOS include directory must exist")
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_dir() {
            collect_headers(&path, headers);
        } else if path.extension().is_some_and(|extension| extension == "h") {
            headers.push(path);
        }
    }
}
