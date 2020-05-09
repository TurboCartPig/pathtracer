use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};
use std::{env, fs::File, path::PathBuf};

fn main() -> Result<(), anyhow::Error> {
    let dir = env::var("OUT_DIR")?;
    let mut file = File::create(&PathBuf::from(&dir).join("gl_bindings.rs"))?;

    Registry::new(Api::Gl, (4, 5), Profile::Core, Fallbacks::All, [])
        .write_bindings(StructGenerator, &mut file)?;

    Ok(())
}
