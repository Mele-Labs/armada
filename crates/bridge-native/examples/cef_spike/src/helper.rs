//! The one binary behind all five Chrome helper apps. Chrome starts it for the GPU, renderer,
//! plugin and alert processes; it loads the framework from the main app and runs that process.

use cef::{args::Args, library_loader::LibraryLoader, *};

fn main() {
    let args = Args::new();
    let loader = LibraryLoader::new(&std::env::current_exe().unwrap(), true);
    assert!(loader.load(), "the Chrome framework did not load");
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    execute_process(Some(args.as_main_args()), None::<&mut App>, std::ptr::null_mut());
}
