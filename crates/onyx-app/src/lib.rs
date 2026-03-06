// --- Onyx Void - Library Entry Point ---
// Required for Android builds: the Makepad app_main!() macro
// generates JNI entry points when compiled as a cdylib (.so).
#![allow(dead_code, unused_imports)]

#[cfg(not(target_os = "android"))]
use mimalloc::MiMalloc;
#[cfg(not(target_os = "android"))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod app;
