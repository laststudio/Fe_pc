#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unknown_lints)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::chunks_exact_to_as_chunks)]
#![allow(clippy::manual_unwrap_or_default)]

use fepc::app::FepcApp;
use fluent_egui::winui::{FluentTheme, RenderBackend, ThemeConfig};

#[cfg(windows)]
#[repr(C)]
pub struct DelayLoadInfo {
    cb: u32,
    pidd: *const std::ffi::c_void,
    ppfn: *mut *mut std::ffi::c_void,
    sz_dll: *const std::ffi::c_char,
    dlp: [usize; 2],
    hmod_cur: *mut std::ffi::c_void,
    pfn_cur: *mut std::ffi::c_void,
    dw_last_error: u32,
}

#[cfg(windows)]
unsafe extern "system" {
    fn LoadLibraryA(name: *const std::ffi::c_char) -> *mut std::ffi::c_void;
}

#[cfg(windows)]
unsafe extern "system" fn win7_delay_hook(
    notify: u32,
    info: *const DelayLoadInfo,
) -> *mut std::ffi::c_void {
    if (notify == 1 || notify == 3) && !info.is_null() {
        let sz_dll = unsafe { (*info).sz_dll };
        if !sz_dll.is_null() {
            let name = unsafe { std::ffi::CStr::from_ptr(sz_dll) }.to_string_lossy();
            if name.eq_ignore_ascii_case("combase.dll") {
                let h = unsafe { LoadLibraryA(c"combase.dll".as_ptr() as *const _) };
                if !h.is_null() {
                    return h;
                }
                return unsafe { LoadLibraryA(c"ole32.dll".as_ptr() as *const _) };
            }
        }
    }
    std::ptr::null_mut()
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub static mut __pfnDliNotifyHook2: unsafe extern "system" fn(u32, *const DelayLoadInfo) -> *mut std::ffi::c_void = win7_delay_hook;

#[cfg(windows)]
#[unsafe(no_mangle)]
pub static mut __pfnDliFailureHook2: unsafe extern "system" fn(u32, *const DelayLoadInfo) -> *mut std::ffi::c_void = win7_delay_hook;


fn main() -> eframe::Result {
    fepc::telemetry::install_panic_hook();
    fepc::workflow::tts::ensure_crypto_provider();

    let config = ThemeConfig::new(FluentTheme::Dark)
        .with_animations(true)
        .with_material_opacity(1.0);

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../fe_logo.png")).ok();
    let mut options = fluent_egui::winui::native_options_with_backend(
        "Fe PC - e听说答案提取器",
        RenderBackend::default(),
    );
    options.viewport = options.viewport
        .with_min_inner_size(eframe::egui::vec2(860.0, 560.0))
        .with_inner_size(eframe::egui::vec2(1040.0, 680.0));
    if let Some(ic) = icon {
        options.viewport = options.viewport.with_icon(ic);
    }

    eframe::run_native(
        "Fe PC - e听说答案提取器",
        options,
        Box::new(move |cc| {
            fluent_egui::winui::install(&cc.egui_ctx, config);

            Ok(Box::new(FepcApp::new(cc)))
        }),
    )
}
