

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "panic occurred in file '{}' at line {}",
            location.file(),
            location.line()
        );
    } else {
        println!("panic occurred but can't get location information...");
    }

    if let Some(message) = info.message() {
        println!("panic message: {}", message);
    } else {
        println!("panic message: <no message>");
    }

    loop {}
}