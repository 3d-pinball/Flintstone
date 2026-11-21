use byteorder::{LittleEndian, WriteBytesExt};
use glam::Mat4;
use image_dds::{ImageFormat, Mipmaps, Quality, Surface, SurfaceRgba8};
use serde_json::Value;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::Command;
use std::{env, fs};

fn main() {
    process_dir(Path::new("./resources/shaders"));
    process_dir(Path::new("./resources/textures"));
}

fn process_dir(dir: &Path) {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);
    dbg!(&dir);
    if !dir.exists() {
        panic!("Directory does not exist: {}", dir.display());
    }
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            if path.extension().map_or(false, |ext| ext == "slang") {
                compile_shader(
                    &path,
                    &out_dir.join(path.with_extension("spv").file_name().unwrap()),
                );
            } else if path.file_stem().map_or(false, |name| {
                name.to_str().map_or(false, |name| name.ends_with("_bc7"))
            }) {
                convert_to_bc7(
                    &path,
                    out_dir
                        .join(
                            path.file_stem()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .strip_suffix("_bc7")
                                .unwrap(),
                        )
                        .with_extension("bc7"),
                );
            }
        }
    }
}

fn compile_shader(path: &Path, out_path: &Path) {
    let success = Command::new("slangc")
        .arg(path)
        .arg("-target")
        .arg("spirv")
        .arg("-profile")
        .arg("spirv_1_4")
        .arg("-emit-spirv-directly")
        .arg("-o")
        .arg(&out_path)
        .status()
        .unwrap()
        .success();
    assert!(success, "Failed to compile shader: {:?}", path);
}

fn convert_to_bc7(path: impl AsRef<Path>, out_path: impl AsRef<Path>) {
    let data = image::open(path).unwrap().into_rgba8();
    let surface = SurfaceRgba8::from_image(&data)
        .encode(
            ImageFormat::BC7RgbaUnormSrgb,
            Quality::Slow,
            Mipmaps::GeneratedAutomatic,
        )
        .unwrap();
    save_bc_to_file(surface, out_path);
}

fn save_bc_to_file(surface: Surface<Vec<u8>>, out_path: impl AsRef<Path>) {
    let mut head = Vec::new();
    let format = match surface.image_format {
        ImageFormat::BC7RgbaUnormSrgb => 0,
        ImageFormat::BC5RgUnorm => 1,
        ImageFormat::BC4RUnorm => 2,
        _ => panic!("Unsupported image format: {:?}", surface.image_format),
    };
    head.write_u32::<LittleEndian>(format).unwrap();
    head.write_u32::<LittleEndian>(surface.width).unwrap();
    head.write_u32::<LittleEndian>(surface.height).unwrap();
    head.write_u32::<LittleEndian>(surface.depth).unwrap();
    head.write_u32::<LittleEndian>(surface.layers).unwrap();
    head.write_u32::<LittleEndian>(surface.mipmaps).unwrap();
    let file = File::create(out_path).unwrap();
    let mut writer = BufWriter::new(file);
    writer.write_all(&head).unwrap();
    writer.write_all(&surface.data).unwrap();
    writer.flush().unwrap();
}