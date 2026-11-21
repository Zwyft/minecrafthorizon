//! Android's activity, as the game's window system needs it.
//!
//! winit's Android backend takes the activity's [`AndroidApp`] when the event loop is built, not
//! from the process: `EventLoop::new` has nothing to attach to. The native entry
//! (`horizon-android`) hands the app over here before the game starts, and
//! `game::runtime::event_loop::new_loop` builds the loop from it.
use std::cell::RefCell;
use winit::event_loop::{EventLoop, EventLoopError};
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::platform::android::activity::AndroidApp;

thread_local! {
    /// The activity's app, on the thread `android_main` runs on.
    static ANDROID_APP: RefCell<Option<AndroidApp>> = const { RefCell::new(None) };
}

/// Hands the activity's app to the game. Called once, on the main thread, before the game starts.
pub fn install(app: AndroidApp) {
    ANDROID_APP.with(|slot| *slot.borrow_mut() = Some(app));
}

/// The event loop the game runs on, carrying the activity's app.
pub(crate) fn event_loop() -> Result<EventLoop<()>, EventLoopError> {
    ANDROID_APP.with(|slot| {
        let app = slot.borrow();
        EventLoop::builder()
            .with_android_app(
                app.as_ref()
                    .expect("horizon_client::android::install runs before the game")
                    .clone(),
            )
            .build()
    })
}
