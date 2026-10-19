fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    // The executable's own icon on Windows (Explorer, taskbar pins, Alt+Tab).
    // The window icon is set separately from assets/icon.png in main.rs.
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=could not embed the Windows icon: {error}");
        }
    }
}
