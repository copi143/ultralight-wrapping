use std::{fs::File, io::BufWriter, path::Path};

use crate::read_c_string;

#[unsafe(no_mangle)]
extern "C" fn ultralightui_save_to_png(path: *const u8, data: *const u8, width: u32, height: u32) {
    let size = (width * height * 4) as usize;
    let data = unsafe { std::slice::from_raw_parts(data, size) };

    let path = Path::new(read_c_string(path));
    let file = File::create(path).unwrap();
    let writer = BufWriter::new(file);

    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(data).unwrap();
}
