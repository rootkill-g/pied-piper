use std::fs;

fn main() -> anyhow::Result<()> {
    // Read the WAT file
    let wat = fs::read_to_string("examples/hello.wat")?;

    // Convert to binary
    let wasm = wasmtime::wat::parse_str(&wat)?;

    // Write the binary
    fs::write("examples/hello.wasm", wasm)?;

    println!("Successfully compiled hello.wat to hello.wasm");
    Ok(())
}
