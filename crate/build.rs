use std::{env, fs, path::Path};

use ambient_design_tokens_core::get_design_tokens;

fn main() {
    let data = get_design_tokens();

    let out_dir = env::var_os("OUT_DIR").unwrap();
    {
        let dest_path = Path::new(&out_dir).join("ambient.css");
        fs::write(
            &dest_path,
            data.iter()
                .map(|x| x.to_css())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    }
    {
        let dest_path = Path::new(&out_dir).join("ambient.rs");
        fs::write(
            &dest_path,
            data.iter()
                .map(|x| format!("pub mod {} {{ {} }}", x.get_name_rust(), x.to_rust()))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    }
    println!("cargo:rerun-if-changed=build.rs");
}
