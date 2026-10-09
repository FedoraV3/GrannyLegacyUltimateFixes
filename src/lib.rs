use windows::Win32::System::Console::AllocConsole;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
unsafe extern "C" fn InitializeASI() {
    unsafe {
        AllocConsole().unwrap();
    }
    println!("Hi");
}
