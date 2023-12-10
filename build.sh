clear
cd crates/web-editor
wasm-pack build --target web $*
cd ../..
cargo run $*