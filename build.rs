fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/pd.ico");
        res.set("ProductName", "PD - Ultra-Lightweight PDF Reader");
        res.set("FileDescription", "Ultra-Lightweight PDF Reader & Desktop Utility");
        res.set("LegalCopyright", "Copyright (c) 2026 IF (if.co.id)");
        let _ = res.compile();
    }
}
