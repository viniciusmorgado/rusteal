use rusteal_ffi::FWeakObjectHandle;

use crate::object_ref::UObjectRef;
use crate::traits::RustObject;

/// Marks a method taking `&mut self`.
pub struct ByMut;

/// Marks a method taking `&self`.
pub struct ByRef;

/// A method of a Rust class that [`method`] turns into a closure: any `fn(&mut self, ...)` or
/// `fn(&self, ...)` of up to nine parameters.
pub trait Method<T, Args, Marker> {
    type Callback;

    fn into_callback(self, object: FWeakObjectHandle) -> Self::Callback;
}

/// A closure calling `function` on `object`, for an input action, a delegate or a timer: the
/// compiler checks the method and its parameters, where a binding by name is checked when the
/// game runs. The object is found again on each call by its index and serial number, so a call
/// after it is gone does nothing, even when another object took its place in memory.
///
/// ```ignore
/// bind_action(&me, self.jump_action(), ETriggerEvent::Started, method(self, Self::jump))?;
/// pickup.on_actor_begin_overlap().add(method(self, Self::begin_overlap))?.detach();
/// ```
pub fn method<T: RustObject, Args, Marker, F: Method<T, Args, Marker>>(
    object: &T,
    function: F,
) -> F::Callback {
    function
        .into_callback(unsafe { UObjectRef::<T>::from_raw(object.object_handle()) }.weak_handle())
}

macro_rules! methods {
    ($($arg:ident $value:ident),*) => {
        impl<T, F, $($arg: 'static),*> Method<T, ($($arg,)*), ByMut> for F
        where
            T: RustObject,
            F: FnMut(&mut T $(, $arg)*) + Send + 'static,
        {
            type Callback = Box<dyn FnMut($($arg),*) + Send>;

            #[allow(unused_variables)]
            fn into_callback(mut self, object: FWeakObjectHandle) -> Self::Callback {
                Box::new(move |$($value: $arg),*| {
                    let handle = UObjectRef::<T>::from_weak(object).raw();

                    if handle.is_null() {
                        return;
                    }

                    if let Ok(mut this) = T::from_object_handle(handle) {
                        self(&mut this $(, $value)*);
                    }
                })
            }
        }

        impl<T, F, $($arg: 'static),*> Method<T, ($($arg,)*), ByRef> for F
        where
            T: RustObject,
            F: FnMut(&T $(, $arg)*) + Send + 'static,
        {
            type Callback = Box<dyn FnMut($($arg),*) + Send>;

            #[allow(unused_variables)]
            fn into_callback(mut self, object: FWeakObjectHandle) -> Self::Callback {
                Box::new(move |$($value: $arg),*| {
                    let handle = UObjectRef::<T>::from_weak(object).raw();

                    if handle.is_null() {
                        return;
                    }

                    if let Ok(this) = T::from_object_handle(handle) {
                        self(&this $(, $value)*);
                    }
                })
            }
        }
    };
}

methods!();
methods!(A a);
methods!(A a, B b);
methods!(A a, B b, C c);
methods!(A a, B b, C c, D d);
methods!(A a, B b, C c, D d, E e);
methods!(A a, B b, C c, D d, E e, G g);
methods!(A a, B b, C c, D d, E e, G g, H h);
methods!(A a, B b, C c, D d, E e, G g, H h, I i);
methods!(A a, B b, C c, D d, E e, G g, H h, I i, J j);
