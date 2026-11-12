mod pad;
mod render;

fn main() {
    match pad::find() {
        Some(device) => println!("Topaz pad at {}", device.display()),
        None => println!("No Topaz pad found."),
    }
}
