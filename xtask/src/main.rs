use devx_cmd::run;
use khonsu_tools::anyhow;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Args {
    BuildWasm,
}

fn main() -> anyhow::Result<()> {
    let args = Args::from_args();
    match args {
        Args::BuildWasm => build_wasm()?,
    };
    Ok(())
}

fn build_wasm() -> Result<(), devx_cmd::Error> {
    println!("Executing cargo build");
    run!(
        "cargo",
        "build",
        "--package",
        "client",
        "--target",
        "wasm32-unknown-unknown",
    )?;

    println!("Executing wasm-bindgen (cargo install wasm-bindgen if you don't have this)");
    run!(
        "wasm-bindgen",
        "target/wasm32-unknown-unknown/debug/client.wasm",
        "--target",
        "web",
        "--out-dir",
        "browser/pkg/",
        "--remove-producers-section"
    )?;

    println!(
        "Build succeeded. ./browser/index.html? can be loaded through any http server that \
         supports wasm."
    );
    println!();
    println!("For example, using `miniserve` (`cargo install miniserve`):");
    println!();
    println!("miniserve browser/");

    Ok(())
}
