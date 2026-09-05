// Run the real host on the process main thread; libtest worker threads cannot own it.
// Other unit-test helpers are compiled by cfg(test) but intentionally unused here.
#![allow(dead_code, unused_imports)]
include!("main.rs");
