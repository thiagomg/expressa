//! Print public Value layout and the inferred map storage's layout on this host.
use expressa::runtime::{Imagem, Value};
fn main() {
    let value = Value::mapa(vec![]);
    let Value::Mapa(map) = value else { unreachable!() };
    println!(
        "{{\"value_bytes\":{},\"value_alignment\":{},\"image_data_bytes\":{},\"map_storage_bytes\":{}}}",
        std::mem::size_of::<Value>(),
        std::mem::align_of::<Value>(),
        std::mem::size_of::<Imagem>(),
        std::mem::size_of_val(map.as_ref()),
    );
}
