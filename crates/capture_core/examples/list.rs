fn main() {
    match capture_core::list_interfaces() {
        Ok(v) => {
            println!("{} interface(s)", v.len());
            for i in v {
                println!("  {:<14} usable={} desc={:?}", i.name, i.usable, i.description);
            }
        }
        Err(e) => eprintln!("error: {e}"),
    }
}
