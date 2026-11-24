//! Hides the system pointer over the render view (`UIPointerInteraction` with a hidden style).
//!
//! `set_cursor_visible(false)` is otherwise a no-op on iOS, so a trackpad or mouse pointer
//! stays drawn over the game whenever UIKit has not locked it. Public API, iPadOS 13.4+.
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{class, declare_class, msg_send, msg_send_id, mutability, ClassType, DeclaredClass};
use objc2_foundation::NSObjectProtocol;
use objc2_ui_kit::UIView;

declare_class!(
    pub(crate) struct WinitPointerHider;

    unsafe impl ClassType for WinitPointerHider {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "WinitPointerHider";
    }

    impl DeclaredClass for WinitPointerHider {}

    unsafe impl NSObjectProtocol for WinitPointerHider {}

    unsafe impl WinitPointerHider {
        // UIPointerInteractionDelegate.pointerInteraction:styleForRegion:
        #[method_id(pointerInteraction:styleForRegion:)]
        fn style_for_region(
            &self,
            _interaction: &AnyObject,
            _region: &AnyObject,
        ) -> Option<Retained<AnyObject>> {
            unsafe { msg_send_id![class!(UIPointerStyle), hiddenPointerStyle] }
        }
    }
);

/// The interaction installed on the view while the cursor is hidden.
pub(crate) struct HiddenPointer {
    interaction: Retained<AnyObject>,
    // UIPointerInteraction keeps its delegate weakly.
    _delegate: Retained<WinitPointerHider>,
}

impl HiddenPointer {
    pub(crate) fn install(view: &UIView, mtm: objc2_foundation::MainThreadMarker) -> Self {
        let delegate: Retained<WinitPointerHider> =
            unsafe { msg_send_id![mtm.alloc::<WinitPointerHider>(), init] };
        let interaction: Retained<AnyObject> = unsafe {
            let alloc: *mut AnyObject = msg_send![class!(UIPointerInteraction), alloc];
            let raw: *mut AnyObject = msg_send![alloc, initWithDelegate: &*delegate];
            Retained::from_raw(raw).expect("UIPointerInteraction init")
        };
        let _: () = unsafe { msg_send![view, addInteraction: &*interaction] };
        Self {
            interaction,
            _delegate: delegate,
        }
    }

    pub(crate) fn remove(self, view: &UIView) {
        let _: () = unsafe { msg_send![view, removeInteraction: &*self.interaction] };
    }
}
