use std::env;

fn main() {
    // Cargo does not automatically propagate an executable's runtime search
    // path from a transitive build dependency. Set RUNPATH on our examples.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        if let Ok(libdir) = env::var("DEP_D2S_FASTDDS_LIBDIR") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{libdir}");
            // Use DT_RPATH so transitive native dependencies can be found too.
            println!("cargo:rustc-link-arg=-Wl,--disable-new-dtags");
        }
    }
}
