//! The lower screen of a dual-screen device (the AYN Thor; any presentation display that is not
//! the default one). `SecondScreen.java` shows a Presentation there, hands its SurfaceView's
//! Surface here and forwards its touches; the game draws a HUD panel into it
//! (`game/second_screen.rs`). The Presentation's window never takes focus, so the controller's
//! keys and sticks keep going to the game's activity.
use jni::{
    JNIEnv,
    objects::{JClass, JObject},
    sys::{jfloat, jint},
};
use std::{
    ffi::c_void,
    ptr::NonNull,
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

#[link(name = "android")]
unsafe extern "C" {
    fn ANativeWindow_fromSurface(env: *mut c_void, surface: *mut c_void) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
}

/// One reference to an `ANativeWindow`, released when its last holder lets go: a panel keeps
/// drawing on valid memory even if its Surface went away under it (it only gets errors).
pub struct NativeWindow(NonNull<c_void>);
// SAFETY: ANativeWindow references are thread-safe (reference counted by the platform).
unsafe impl Send for NativeWindow {}
unsafe impl Sync for NativeWindow {}
impl NativeWindow {
    pub fn ptr(&self) -> NonNull<c_void> {
        self.0
    }
}
impl Drop for NativeWindow {
    fn drop(&mut self) {
        unsafe { ANativeWindow_release(self.0.as_ptr()) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

/// A touch on the second screen, in its surface's pixels.
#[derive(Clone, Copy, Debug)]
pub struct Touch {
    pub phase: Phase,
    pub at: [f32; 2],
}

/// The surface to draw on: a new generation for every new Surface, its window and size.
#[derive(Clone)]
pub struct Surface {
    pub generation: u64,
    pub window: Arc<NativeWindow>,
    pub size: [u32; 2],
}

struct Shared {
    surface: Option<Surface>,
    generations: u64,
    /// The generation the game draws on now; a destroyed Surface waits for it to let go.
    drawing: Option<u64>,
    touches: Vec<Touch>,
}
static SHARED: Mutex<Shared> = Mutex::new(Shared {
    surface: None,
    generations: 0,
    drawing: None,
    touches: Vec::new(),
});
static RELEASED: Condvar = Condvar::new();
/// Touches kept while nothing reads them (the launcher): a few gestures' worth.
const TOUCH_LIMIT: usize = 256;

fn lock() -> MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The second screen's surface, while it has one.
pub fn surface() -> Option<Surface> {
    lock().surface.clone()
}

/// The game draws on `generation` now (`None`: on nothing).
pub fn drawing(generation: Option<u64>) {
    let mut shared = lock();
    if shared.drawing != generation {
        shared.drawing = generation;
        RELEASED.notify_all();
    }
}

/// The touches since the last call.
pub fn take_touches() -> Vec<Touch> {
    std::mem::take(&mut lock().touches)
}

#[unsafe(export_name = "Java_dev_horizon_android_SecondScreen_nativeSurface")]
extern "system" fn native_surface(
    env: JNIEnv<'_>,
    _: JClass<'_>,
    surface: JObject<'_>,
    width: jint,
    height: jint,
) {
    let size = [width.max(1) as u32, height.max(1) as u32];
    if surface.is_null() {
        // SurfaceHolder.Callback.surfaceDestroyed: nothing may draw on the Surface once this
        // returns. A game loop that is not running draws nothing, so the wait is bounded.
        let mut shared = lock();
        let Some(gone) = shared.surface.take().map(|s| s.generation) else {
            return;
        };
        shared.touches.clear();
        let _ = RELEASED.wait_timeout_while(shared, Duration::from_millis(500), |s| {
            s.drawing == Some(gone)
        });
        return;
    }
    let raw = unsafe { ANativeWindow_fromSurface(env.get_raw().cast(), surface.as_raw().cast()) };
    let Some(window) = NonNull::new(raw).map(NativeWindow) else {
        log::warn!("Second screen: the Surface has no native window");
        return;
    };
    let mut shared = lock();
    match &mut shared.surface {
        // surfaceChanged on the same window: only the size is new (this extra reference drops).
        Some(current) if current.window.ptr() == window.ptr() => current.size = size,
        _ => {
            shared.generations += 1;
            shared.surface = Some(Surface {
                generation: shared.generations,
                window: Arc::new(window),
                size,
            });
        }
    }
}

#[unsafe(export_name = "Java_dev_horizon_android_SecondScreen_nativeTouch")]
extern "system" fn native_touch(
    _: JNIEnv<'_>,
    _: JClass<'_>,
    phase: jint,
    _id: jint,
    x: jfloat,
    y: jfloat,
) {
    let phase = match phase {
        0 => Phase::Started,
        1 => Phase::Moved,
        2 => Phase::Ended,
        _ => Phase::Cancelled,
    };
    let mut shared = lock();
    if shared.touches.len() < TOUCH_LIMIT {
        shared.touches.push(Touch { phase, at: [x, y] });
    }
}

/// The panel on the second screen, on the game's thread (`game/second_screen.rs` draws it).
pub struct Shown {
    pub panel: horizon_client::gfx::panel::Panel,
    pub generation: u64,
    /// The window the panel's surface was made on. Declared after `panel`, so it is released
    /// after the surface is dropped.
    pub _window: Arc<NativeWindow>,
    pub drawn: Option<std::time::Instant>,
}

thread_local! {
    pub static SHOWN: std::cell::RefCell<Option<Shown>> = const { std::cell::RefCell::new(None) };
    /// The last generation no panel could be made for: not retried until a new Surface comes.
    pub static FAILED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// A wgpu surface on the second screen's window. The `Shown` it goes into keeps the window
/// referenced for as long as the surface.
pub fn panel_surface(
    instance: &wgpu::Instance,
    surface: &Surface,
) -> horizon_client::Result<wgpu::Surface<'static>> {
    use wgpu::rwh::{
        AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
    };
    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle: Some(RawDisplayHandle::Android(AndroidDisplayHandle::new())),
        raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(
            surface.window.ptr(),
        )),
    };
    // SAFETY: the window is a live ANativeWindow reference, held by the caller's `Shown` (with
    // the panel that owns this surface) until after the surface is dropped.
    Ok(unsafe { instance.create_surface_unsafe(target) }?)
}

/// Drop the panel (a new Surface, a new renderer, back to the launcher).
pub fn close() {
    let shown = SHOWN.take();
    drop(shown);
    drawing(None);
}
