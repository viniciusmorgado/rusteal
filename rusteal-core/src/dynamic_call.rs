use rusteal_ffi::{FPropertyHandle, UFunctionHandle, UObjectHandle};

use crate::error::{RustealError, RustealResult, check_ffi};
use crate::ffi_dispatch::{self, NATIVE_PTR_NULL, NativePtr, native_ptr_is_null};
use crate::object_ref::UObjectRef;
use crate::traits::UeClass;

pub struct DynamicCall {
    obj: UObjectHandle,
    func: UFunctionHandle,
    params: NativePtr,
    delegate: FPropertyHandle,
}

impl DynamicCall {
    pub fn new(obj: &UObjectRef<impl UeClass>, func_name: &str) -> RustealResult<Self> {
        let h = obj.checked()?.raw();

        let func = unsafe {
            ffi_dispatch::reflection_find_function(h, func_name.as_ptr(), func_name.len() as u32)
        };

        if func.is_null() {
            return Err(RustealError::FunctionNotFound(func_name.to_string()));
        }

        let params = unsafe { ffi_dispatch::reflection_alloc_params(func) };

        Ok(DynamicCall {
            obj: h,
            func,
            params,
            delegate: FPropertyHandle::null(),
        })
    }

    pub fn for_delegate(
        obj: &UObjectRef<impl UeClass>,
        delegate: FPropertyHandle,
    ) -> RustealResult<Self> {
        let h = obj.checked()?.raw();

        if delegate.is_null() {
            return Err(RustealError::PropertyNotFound("delegate".to_string()));
        }

        let func = unsafe { ffi_dispatch::reflection_get_delegate_signature(delegate) };

        if func.is_null() {
            return Err(RustealError::TypeMismatch);
        }

        let params = unsafe { ffi_dispatch::reflection_alloc_params(func) };

        Ok(DynamicCall {
            obj: h,
            func,
            params,
            delegate,
        })
    }

    pub fn set<T: Copy>(&mut self, name: &str, value: T) -> RustealResult<()> {
        let (prop, offset) = self.find_param(name)?;
        let _ = prop;

        unsafe {
            ffi_dispatch::native_mem_write(self.params, offset as usize, value);
        }

        Ok(())
    }

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

    pub fn broadcast(self) -> RustealResult<()> {
        if self.delegate.is_null() {
            return Err(RustealError::InvalidOperation("not a delegate call".into()));
        }

        check_ffi(unsafe {
            ffi_dispatch::delegate_broadcast_multicast(self.obj, self.delegate, self.params)
        })
    }

    pub fn call(mut self) -> RustealResult<DynamicCallResult> {
        let code =
            unsafe { ffi_dispatch::reflection_call_function(self.obj, self.func, self.params) };

        check_ffi(code)?;

        let result = DynamicCallResult {
            func: self.func,
            params: self.params,
        };

        self.params = NATIVE_PTR_NULL;

        Ok(result)
    }

    fn find_param(&self, name: &str) -> RustealResult<(FPropertyHandle, u32)> {
        let prop = unsafe {
            ffi_dispatch::reflection_get_function_param(self.func, name.as_ptr(), name.len() as u32)
        };

        if prop.is_null() {
            return Err(RustealError::PropertyNotFound(name.to_string()));
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
    pub fn get<T: Copy>(&self, name: &str) -> RustealResult<T> {
        let prop = unsafe {
            ffi_dispatch::reflection_get_function_param(self.func, name.as_ptr(), name.len() as u32)
        };

        if prop.is_null() {
            return Err(RustealError::PropertyNotFound(name.to_string()));
        }

        let offset = unsafe { ffi_dispatch::reflection_get_property_offset(prop) };

        let value = unsafe { ffi_dispatch::native_mem_read(self.params, offset as usize) };

        Ok(value)
    }
}

impl Drop for DynamicCallResult {
    fn drop(&mut self) {
        if !native_ptr_is_null(self.params) {
            unsafe { ffi_dispatch::reflection_free_params(self.func, self.params) };
        }
    }
}
