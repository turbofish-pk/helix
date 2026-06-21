//! The event system makes use of global to decouple different systems.
//! However, this can cause problems for the integration test system because
//! it runs multiple helix applications in parallel. Making the globals
//! thread-local does not work because a applications can/does have multiple
//! runtime threads. Instead this crate implements a similar notion to a thread
//! local but instead of being local to a single thread, the statics are local to
//! a single tokio-runtime. The implementation requires locking so it's not exactly efficient.
//!
//! Therefore this function is only enabled during integration tests and behaves like
//! a normal static otherwise. I would prefer this module to be fully private and to only
//! export the macro but the macro still need to construct these internals so it's marked
//! `doc(hidden)` instead

use std::ops::Deref;

pub struct RuntimeLocal<T: 'static> {
    /// inner API used in the macro, not part of public API
    #[doc(hidden)]
    pub __data: T,
}

impl<T> Deref for RuntimeLocal<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.__data
    }
}

#[macro_export]
macro_rules! runtime_local {
    ($($(#[$attr:meta])* $vis: vis static $name:ident: $ty: ty = $init: expr;)*) => {
        $($(#[$attr])* $vis static $name: $crate::runtime::RuntimeLocal<$ty> = $crate::runtime::RuntimeLocal {
            __data: $init
        };)*
    };
}
