# Horizon UIKit input patch

Vendored from the existing dependency winit 0.30.13, crates.io archive:
https://static.crates.io/crates/winit/winit-0.30.13.crate

Archive SHA-256 (also the original Cargo.lock checksum):
`a6755fa58a9f8350bd1e472d4c3fcc25f824ec358933bba33306d0b63df5978d`
Upstream revision: `e9809ef54b18499bb4f2cac945719ecc2a61061b`.
The upstream Apache-2.0 license and copyright notices are retained in LICENSE.

The patch is confined to Cargo.toml (UIKit binding features), src/lib.rs (host mapping tests),
and src/platform_impl/ios/{mod,view,view_controller,window,event_loop,scene,hardware_keyboard,hardware_mouse,hidden_pointer,pointer_lock_watch}.rs.
`src/event.rs` also provides the iOS-only virtual-key constructor described below. All other files are the unmodified published crate. Root Cargo.toml patches crates.io so local
and CI builds use the same input backend; Cargo.lock removes only winit's registry source/checksum.

UIKit UIKey physical HID usages produce ordinary winit keyboard events, with layout text,
modifiers, release/cancel events, and tracked key releases on responder loss. The view remains a
first responder without showing a software keyboard during gameplay; set_ime_allowed switches
its input view. UIKit text insertion/deletion still handles the software keyboard.

UIViewController.prefersPointerLocked and setNeedsUpdateOfPrefersPointerLocked request capture;
a nil scene pointerLockState reports unsupported. GameController GCMouse handlers provide
relative movement, captured buttons, and scroll. New mice are attached during redraw requests;
disconnection releases buttons. Handler queues remain Apple's documented main-thread default.
Captured indirect pointer touches are suppressed so one click does not also hit touch controls.
`prefersPointerLocked` is only a preference, so each frame the window compares it with the scene's
`pointerLockState.isLocked`: a wanted lock that is missing is re-requested every 250 ms, up to
8 times, then a warning is logged once (`pointer_lock_watch.rs`, host-tested). `set_cursor_visible(false)`
installs a `UIPointerInteraction` whose delegate returns `UIPointerStyle.hiddenPointerStyle`
(`hidden_pointer.rs`), hiding the system pointer while UIKit has not locked it; `true` removes it.

iPhone has no UIKit pointer lock (`pointerLockState` is nil). With a GameController mouse attached,
`set_cursor_grab(Locked)` then succeeds as a *virtual* lock: raw deltas, buttons and scroll are
forwarded as when locked and indirect-pointer touches are suppressed, but the system pointer stays
visible. With no mouse attached it still reports NotSupported so touch input is unchanged.

The app requires iOS 16; pointer lock/GCMouse require iPadOS 14 and UIKey requires iOS 13.4.
UIKit ultimately decides whether to honor lock requests. Touch/controller input remains the
fallback when no supported mouse is present. No cursor warping or private API is used.

Remove this patch when the project's chosen upstream winit version implements equivalent
UIKit hardware keyboard and relative mouse/pointer-lock behavior, after physical-device tests.

## Scene lifecycle

SDK-27 apps must use UIScene.
The game plist now declares one UIWindowScene with WinitSceneDelegate; the event loop registers
that class before UIApplicationMain and defers launch completion until scene connection. Scene
activation/background callbacks drive Resumed/Suspended, focus, and occlusion. Windows are
created with initWithWindowScene. Legacy clients without a scene manifest keep their original
notification path. This is lifecycle adoption, not an SDK-version spoof or runtime-check bypass.

Apple guidance: https://developer.apple.com/documentation/uikit/transitioning-to-the-uikit-scene-based-life-cycle
and https://developer.apple.com/videos/play/wwdc2026/278/.

Named hardware navigation keys do not produce text, even when UIKit supplies symbolic
UIKeyInput names. Their physical/logical key actions are retained; printable layout text
and space continue to produce character input. Host regression tests cover this mapping.

## Render-view coordinates

`inner_size`, layout resize events and content-scale changes report the full render-view
bounds multiplied by its content scale. Safe-area insets must not shrink the Metal drawable,
which fills that view: doing so stretches the image and shifts rendered button locations away
from their input regions after renderer recreation. Touches and indirect-pointer clicks use
view-local coordinates, matching hover events, including across orientation changes.

## Custom touch key transport

`KeyEvent::from_virtual_key` creates an iOS-only physical key press/release without invented text or repeat events. Custom touch bindings use it to reach the same menu/edit-box handlers as physical keys. Horizon marks this dispatch as touch input so it neither claims hardware ownership nor adds virtual keys to the physical keyboard ledger.
