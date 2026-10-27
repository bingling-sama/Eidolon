//! DirectComposition plumbing for a per-pixel-alpha preview window on Windows.
//!
//! An HWND swap chain (`CreateSwapChainForHwnd`) ignores the alpha channel, so a transparent
//! window needs the surface to come from a DirectComposition visual
//! (`CreateSwapChainForComposition`). wgpu creates and attaches the swap chain itself when the
//! surface is built from a visual; this module only owns the composition device, target and
//! visual that outlive it.

use std::ffi::c_void;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Owns the composition objects backing a transparent preview window.
pub struct CompositionVisual {
    _device: IDCompositionDevice,
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
}

/// Create a composition device without attaching it to a window.
///
/// Used as a preflight: the window is only created without a redirection surface when the
/// composition path has a chance of working.
pub fn device_available() -> bool {
    let device: Result<IDCompositionDevice, _> = unsafe { DCompositionCreateDevice2(None) };
    device.is_ok()
}

impl CompositionVisual {
    /// Attach a composition visual to `window` and make it the window's root content.
    pub fn attach(window: &impl HasWindowHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let handle = window
            .window_handle()
            .map_err(|e| format!("no window handle: {e}"))?
            .as_raw();
        let RawWindowHandle::Win32(win32) = handle else {
            return Err("expected a Win32 window handle".into());
        };
        // The HWND is only read, never owned, so a transient wrapper is fine.
        let hwnd = HWND(win32.hwnd.get() as *mut c_void);

        unsafe {
            let device: IDCompositionDevice =
                DCompositionCreateDevice2(None).map_err(|e| format!("dcomp device: {e}"))?;
            let target = device
                .CreateTargetForHwnd(hwnd, true)
                .map_err(|e| format!("dcomp target: {e}"))?;
            let visual = device
                .CreateVisual()
                .map_err(|e| format!("dcomp visual: {e}"))?;
            target
                .SetRoot(&visual)
                .map_err(|e| format!("dcomp set root: {e}"))?;
            device.Commit().map_err(|e| format!("dcomp commit: {e}"))?;
            Ok(Self {
                _device: device,
                _target: target,
                visual,
            })
        }
    }

    /// Raw `IDCompositionVisual` pointer for [`wgpu::SurfaceTargetUnsafe::CompositionVisual`].
    pub fn as_ptr(&self) -> *mut c_void {
        use windows::core::Interface;
        self.visual.as_raw()
    }

    /// Push pending visual changes to the compositor.
    ///
    /// wgpu attaches the swap chain with `IDCompositionVisual::SetContent` but leaves the commit
    /// to the application, so this must run after every surface (re)configuration.
    pub fn commit(&self) {
        if let Err(err) = unsafe { self._device.Commit() } {
            log::error!("DirectComposition commit failed: {err}");
        }
    }
}
