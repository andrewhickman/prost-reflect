use std::io::Result;

use prost::Message;
use prost_types::FileDescriptorSet;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=src/well_known_types.bin");

    let mut wkt_bin = std::fs::read("./src/well_known_types.bin")?;
    let mut wkt_set = FileDescriptorSet::decode(wkt_bin.as_slice())?;
    for mut file in wkt_set.file.iter_mut() {
        file.source_code_info = None; // clear source code info
    }

    wkt_bin.clear();
    wkt_set.encode(&mut wkt_bin)?;

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("well_known_types.bin");
    std::fs::write(dest_path, wkt_bin)?;
    Ok(())
}
