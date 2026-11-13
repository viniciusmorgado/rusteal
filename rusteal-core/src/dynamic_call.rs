use std::any::Any;

use rusteal_ffi::{FNameHandle, FPropertyHandle, UFunctionHandle, UObjectHandle};

use crate::error::{RustealError, RustealResult, check_ffi};
use crate::ffi_dispatch::{self, NATIVE_PTR_NULL, NativePtr, native_ptr_is_null};
use crate::fname::FName;
use crate::object_ref::UObjectRef;
use crate::traits::UeClass;

pub struct DynamicCall {
    obj: UObjectHandle,
    func: UFunctionHandle,
    params: NativePtr,
    delegate: FPropertyHandle,
}

impl DynamicCall {
    #[track_caller]
    pub fn new(obj: &UObjectRef<impl UeClass>, func_name: &str) -> RustealResult<Self> {
        Self::on_object(obj.checked()?.raw(), func_name)
    }

    #[track_caller]
    pub(crate) fn on_object(h: UObjectHandle, func_name: &str) -> RustealResult<Self> {
        let func = unsafe {
            ffi_dispatch::reflection_find_function(h, func_name.as_ptr(), func_name.len() as u32)
        };

        if func.is_null() {
            return Err(RustealError::function_not_found(func_name.to_string()).on_handle(h));
        }

        let params = unsafe { ffi_dispatch::reflection_alloc_params(func) };

        Ok(DynamicCall {
            obj: h,
            func,
            params,
            delegate: FPropertyHandle::null(),
        })
    }

    #[track_caller]
    pub fn for_delegate(
        obj: &UObjectRef<impl UeClass>,
        delegate: FPropertyHandle,
    ) -> RustealResult<Self> {
        let h = obj.checked()?.raw();

        if delegate.is_null() {
            return Err(RustealError::property_not_found("delegate").on_handle(h));
        }

        let func = unsafe { ffi_dispatch::reflection_get_delegate_signature(delegate) };

        if func.is_null() {
            return Err(RustealError::type_mismatch()
                .context("the property is not a delegate")
                .on_handle(h));
        }

        let params = unsafe { ffi_dispatch::reflection_alloc_params(func) };

        Ok(DynamicCall {
            obj: h,
            func,
            params,
            delegate,
        })
    }

    #[track_caller]
    pub fn set<T: Copy + 'static>(&mut self, name: &str, value: T) -> RustealResult<()> {
        let (prop, offset) = self.find_param(name)?;

        if let Some(handle) = as_fname_handle(&value) {
            return check_ffi(unsafe {
                ffi_dispatch::property_set_fname(params_as_container(self.params), prop, handle)
            });
        }

        unsafe {
            ffi_dispatch::native_mem_write(self.params, offset as usize, value);
        }

        Ok(())
    }

    #[track_caller]
    pub fn set_value<T: crate::dynamic_value::DynamicValue>(
        &mut self,
        name: &str,
        value: &T,
    ) -> RustealResult<()> {
        let (prop, _) = self.find_param(name)?;

        value.write_to(params_as_container(self.params), prop)
    }

    #[track_caller]
    pub fn set_struct<T: crate::traits::UeStruct>(
        &mut self,
        name: &str,
        value: &crate::containers::OwnedStruct<T>,
    ) -> RustealResult<()> {
        let (_, offset) = self.find_param(name)?;

        check_ffi(unsafe {
            ffi_dispatch::reflection_copy_struct(
                T::static_struct(),
                self.params.add(offset as usize),
                value.as_bytes().as_ptr(),
            )
        })
    }

    #[track_caller]
    pub fn set_struct_ref<T: crate::traits::UeStruct>(
        &mut self,
        name: &str,
        value: &crate::struct_ref::UStructRef<T>,
    ) -> RustealResult<()> {
        let (_, offset) = self.find_param(name)?;

        check_ffi(unsafe {
            ffi_dispatch::reflection_copy_struct(
                T::static_struct(),
                self.params.add(offset as usize),
                value.as_ptr().0 as *const u8,
            )
        })
    }

    #[track_caller]
    pub fn broadcast(self) -> RustealResult<()> {
        if self.delegate.is_null() {
            return Err(RustealError::invalid_operation("not a delegate call"));
        }

        check_ffi(unsafe {
            ffi_dispatch::delegate_broadcast_multicast(self.obj, self.delegate, self.params)
        })
    }

    #[track_caller]
    pub fn execute(self) -> RustealResult<()> {
        if self.delegate.is_null() {
            return Err(RustealError::invalid_operation("not a delegate call"));
        }

        check_ffi(unsafe {
            ffi_dispatch::delegate_execute_delegate(self.obj, self.delegate, self.params)
        })
    }

    #[track_caller]
    pub fn call(mut self) -> RustealResult<DynamicCallResult> {
        let code =
            unsafe { ffi_dispatch::reflection_call_function(self.obj, self.func, self.params) };

        check_ffi(code).map_err(|e| {
            e.context(format!("calling {}", function_name(self.func)))
                .on_handle(self.obj)
        })?;

        let result = DynamicCallResult {
            func: self.func,
            params: self.params,
        };

        self.params = NATIVE_PTR_NULL;

        Ok(result)
    }

    #[track_caller]
    fn find_param(&self, name: &str) -> RustealResult<(FPropertyHandle, u32)> {
        let prop = unsafe {
            ffi_dispatch::reflection_get_function_param(self.func, name.as_ptr(), name.len() as u32)
        };

        if prop.is_null() {
            return Err(missing_param(self.func, name).on_handle(self.obj));
        }

        let offset = unsafe { ffi_dispatch::reflection_get_property_offset(prop) };

        Ok((prop, offset))
    }
}

impl Drop for DynamicCall {
    fn drop(&mut self) {
        if !native_ptr_is_null(self.params) {
            unsafe { ffi_dispatch::reflection_free_params(self.func, self.params) };
        }
    }
}

pub struct DynamicCallResult {
    func: UFunctionHandle,
    params: NativePtr,
}

impl DynamicCallResult {
    #[track_caller]
    pub fn value<T: crate::dynamic_value::DynamicValue>(&self, name: &str) -> RustealResult<T> {
        let prop = unsafe {
            ffi_dispatch::reflection_get_function_param(self.func, name.as_ptr(), name.len() as u32)
        };

        if prop.is_null() {
            return Err(missing_param(self.func, name));
        }

        T::read_from(params_as_container(self.params), prop)
    }

    #[track_caller]
    pub fn get<T: Copy + 'static>(&self, name: &str) -> RustealResult<T> {
        let prop = unsafe {
            ffi_dispatch::reflection_get_function_param(self.func, name.as_ptr(), name.len() as u32)
        };

        if prop.is_null() {
            return Err(missing_param(self.func, name));
        }

        if is_fname::<T>() {
            let mut handle = FNameHandle::default();

            check_ffi(unsafe {
                ffi_dispatch::property_get_fname(
                    params_as_container(self.params),
                    prop,
                    &mut handle,
                )
            })?;

            let name = FName(handle);

            return (&name as &dyn Any)
                .downcast_ref::<T>()
                .or_else(|| (&handle as &dyn Any).downcast_ref::<T>())
                .copied()
                .ok_or_else(|| RustealError::type_mismatch());
        }

        let offset = unsafe { ffi_dispatch::reflection_get_property_offset(prop) };

        let value = unsafe { ffi_dispatch::native_mem_read(self.params, offset as usize) };

        Ok(value)
    }
}

fn function_name(func: UFunctionHandle) -> String {
    crate::error::object_name(UObjectHandle(func.0)).unwrap_or_else(|| "a function".to_owned())
}

#[track_caller]
fn missing_param(func: UFunctionHandle, name: &str) -> RustealError {
    RustealError::property_not_found(format!("parameter {name} of {}", function_name(func)))
}

fn is_fname<T: 'static>() -> bool {
    let id = std::any::TypeId::of::<T>();

    id == std::any::TypeId::of::<FNameHandle>() || id == std::any::TypeId::of::<FName>()
}

fn as_fname_handle<T: 'static>(value: &T) -> Option<FNameHandle> {
    let value = value as &dyn Any;

    value
        .downcast_ref::<FNameHandle>()
        .copied()
        .or_else(|| value.downcast_ref::<FName>().map(FName::handle))
}

fn params_as_container(params: NativePtr) -> UObjectHandle {
    UObjectHandle(params as *mut std::ffi::c_void)
}

impl Drop for DynamicCallResult {
    fn drop(&mut self) {
        if !native_ptr_is_null(self.params) {
            unsafe { ffi_dispatch::reflection_free_params(self.func, self.params) };
        }
    }
}
