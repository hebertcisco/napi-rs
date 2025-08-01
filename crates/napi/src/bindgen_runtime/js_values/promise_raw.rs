use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr;

#[cfg(all(feature = "napi4", feature = "tokio_rt"))]
use crate::bindgen_runtime::Promise;
use crate::{
  bindgen_prelude::{
    FromNapiValue, JsObjectValue, Result, ToNapiValue, TypeName, ValidateNapiValue,
  },
  check_status, sys, Env, Error, JsValue, Value, ValueType,
};

#[derive(Clone, Copy)]
pub struct PromiseRaw<'env, T> {
  pub(crate) inner: sys::napi_value,
  env: sys::napi_env,
  _phantom: &'env PhantomData<T>,
}

impl<'env, T> JsValue<'env> for PromiseRaw<'env, T> {
  fn value(&self) -> Value {
    Value {
      env: self.env,
      value: self.inner,
      value_type: ValueType::Object,
    }
  }
}

impl<'env, T> JsObjectValue<'env> for PromiseRaw<'env, T> {}

impl<T> TypeName for PromiseRaw<'_, T> {
  fn type_name() -> &'static str {
    "Promise"
  }

  fn value_type() -> crate::ValueType {
    crate::ValueType::Object
  }
}

impl<T> ValidateNapiValue for PromiseRaw<'_, T> {
  unsafe fn validate(
    env: napi_sys::napi_env,
    napi_val: napi_sys::napi_value,
  ) -> Result<napi_sys::napi_value> {
    validate_promise(env, napi_val)
  }
}

impl<T> FromNapiValue for PromiseRaw<'_, T> {
  unsafe fn from_napi_value(env: napi_sys::napi_env, value: napi_sys::napi_value) -> Result<Self> {
    Ok(PromiseRaw::new(env, value))
  }
}

impl<T> PromiseRaw<'_, T> {
  pub(crate) fn new(env: sys::napi_env, inner: sys::napi_value) -> Self {
    Self {
      inner,
      env,
      _phantom: &PhantomData,
    }
  }
}

impl<'env, T: FromNapiValue> PromiseRaw<'env, T> {
  /// Promise.then method
  pub fn then<Callback, U>(&self, cb: Callback) -> Result<PromiseRaw<'env, U>>
  where
    U: ToNapiValue,
    Callback: 'static + FnOnce(CallbackContext<T>) -> Result<U>,
  {
    let mut then_fn = ptr::null_mut();
    const THEN: &[u8; 5] = b"then\0";
    check_status!(unsafe {
      sys::napi_get_named_property(self.env, self.inner, THEN.as_ptr().cast(), &mut then_fn)
    })?;
    let mut then_callback = ptr::null_mut();
    let executed = Box::into_raw(Box::new(false));
    let rust_cb = Box::into_raw(Box::new((cb, executed)));
    check_status!(
      unsafe {
        sys::napi_create_function(
          self.env,
          THEN.as_ptr().cast(),
          4,
          Some(raw_promise_then_callback::<T, U, Callback>),
          rust_cb.cast(),
          &mut then_callback,
        )
      },
      "Create then function for PromiseRaw failed"
    )?;
    let mut new_promise = ptr::null_mut();
    check_status!(
      unsafe {
        sys::napi_call_function(
          self.env,
          self.inner,
          then_fn,
          1,
          [then_callback].as_ptr(),
          &mut new_promise,
        )
      },
      "Call the PromiseRaw::then failed"
    )?;

    // use `napi_wrap` to trigger the finalizer after the Promise is GCed
    // Note: we don't use `napi_add_finalizer` here because it requires `napi5`
    check_status!(
      unsafe {
        sys::napi_wrap(
          self.env,
          new_promise,
          executed.cast(),
          Some(promise_callback_finalizer::<T, U, Callback>),
          rust_cb.cast(),
          ptr::null_mut(),
        )
      },
      "Wrap finalizer for PromiseRaw failed"
    )?;

    Ok(PromiseRaw::<U> {
      env: self.env,
      inner: new_promise,
      _phantom: &PhantomData,
    })
  }

  /// Promise.catch method
  pub fn catch<E, U, Callback>(&self, cb: Callback) -> Result<PromiseRaw<'env, U>>
  where
    E: FromNapiValue,
    U: ToNapiValue,
    Callback: 'static + FnOnce(CallbackContext<E>) -> Result<U>,
  {
    let mut catch_fn = ptr::null_mut();
    const CATCH: &[u8; 6] = b"catch\0";
    check_status!(unsafe {
      sys::napi_get_named_property(self.env, self.inner, CATCH.as_ptr().cast(), &mut catch_fn)
    })?;
    let mut catch_callback = ptr::null_mut();
    let executed = Box::into_raw(Box::new(false));
    let rust_cb = Box::into_raw(Box::new((cb, executed)));
    check_status!(
      unsafe {
        sys::napi_create_function(
          self.env,
          CATCH.as_ptr().cast(),
          5,
          Some(raw_promise_catch_callback::<E, U, Callback>),
          rust_cb.cast(),
          &mut catch_callback,
        )
      },
      "Create catch function for PromiseRaw failed"
    )?;
    let mut new_promise = ptr::null_mut();
    check_status!(
      unsafe {
        sys::napi_call_function(
          self.env,
          self.inner,
          catch_fn,
          1,
          [catch_callback].as_mut_ptr().cast(),
          &mut new_promise,
        )
      },
      "Call the PromiseRaw::catch failed"
    )?;

    // use `napi_wrap` to trigger the finalizer after the Promise is GCed
    // Note: we don't use `napi_add_finalizer` here because it requires `napi5`
    check_status!(
      unsafe {
        sys::napi_wrap(
          self.env,
          new_promise,
          executed.cast(),
          Some(promise_callback_finalizer::<E, U, Callback>),
          rust_cb.cast(),
          ptr::null_mut(),
        )
      },
      "Wrap finalizer for PromiseRaw failed"
    )?;

    Ok(PromiseRaw::<U> {
      env: self.env,
      inner: new_promise,
      _phantom: &PhantomData,
    })
  }

  /// Promise.finally method
  pub fn finally<U, Callback>(&mut self, cb: Callback) -> Result<PromiseRaw<'env, T>>
  where
    U: ToNapiValue,
    Callback: 'static + FnOnce(Env) -> Result<U>,
  {
    let mut then_fn = ptr::null_mut();
    const FINALLY: &[u8; 8] = b"finally\0";

    check_status!(unsafe {
      sys::napi_get_named_property(self.env, self.inner, FINALLY.as_ptr().cast(), &mut then_fn)
    })?;
    let mut then_callback = ptr::null_mut();
    let rust_cb = Box::into_raw(Box::new(cb));
    check_status!(
      unsafe {
        sys::napi_create_function(
          self.env,
          FINALLY.as_ptr().cast(),
          7,
          Some(raw_promise_finally_callback::<U, Callback>),
          rust_cb.cast(),
          &mut then_callback,
        )
      },
      "Create then function for PromiseRaw failed"
    )?;
    let mut new_promise = ptr::null_mut();
    check_status!(
      unsafe {
        sys::napi_call_function(
          self.env,
          self.inner,
          then_fn,
          1,
          [then_callback].as_ptr(),
          &mut new_promise,
        )
      },
      "Call then callback on PromiseRaw failed"
    )?;

    Ok(Self {
      env: self.env,
      inner: new_promise,
      _phantom: &PhantomData,
    })
  }

  #[cfg(all(feature = "napi4", feature = "tokio_rt"))]
  /// Convert `PromiseRaw<T>` to `Promise<T>`
  ///
  /// So you can await the Promise in Rust
  pub fn into_sendable_promise(self) -> Result<Promise<T>> {
    unsafe { Promise::from_napi_value(self.env, self.inner) }
  }
}

pub(crate) fn validate_promise(
  env: napi_sys::napi_env,
  napi_val: napi_sys::napi_value,
) -> Result<sys::napi_value> {
  let mut is_promise = false;
  check_status!(
    unsafe { crate::sys::napi_is_promise(env, napi_val, &mut is_promise) },
    "Failed to check if value is promise"
  )?;
  if !is_promise {
    let mut deferred = ptr::null_mut();
    let mut promise = ptr::null_mut();
    check_status!(
      unsafe { crate::sys::napi_create_promise(env, &mut deferred, &mut promise) },
      "Failed to create promise"
    )?;
    const INVALID_ARG: &[u8; 11] = b"InvalidArg\0";
    let mut err = ptr::null_mut();
    let mut code = ptr::null_mut();
    let mut message = ptr::null_mut();
    check_status!(
      unsafe {
        crate::sys::napi_create_string_utf8(env, INVALID_ARG.as_ptr().cast(), 10, &mut code)
      },
      "Failed to create error message"
    )?;
    check_status!(
      unsafe {
        crate::sys::napi_create_string_utf8(
          env,
          c"Expected Promise object".as_ptr().cast(),
          23,
          &mut message,
        )
      },
      "Failed to create error message"
    )?;
    check_status!(
      unsafe { crate::sys::napi_create_error(env, code, message, &mut err) },
      "Failed to create rejected error"
    )?;
    check_status!(
      unsafe { crate::sys::napi_reject_deferred(env, deferred, err) },
      "Failed to reject promise in validate"
    )?;
    return Ok(promise);
  }
  Ok(ptr::null_mut())
}

unsafe extern "C" fn raw_promise_then_callback<T, U, Cb>(
  env: sys::napi_env,
  cbinfo: sys::napi_callback_info,
) -> sys::napi_value
where
  T: FromNapiValue,
  U: ToNapiValue,
  Cb: FnOnce(CallbackContext<T>) -> Result<U>,
{
  handle_then_callback::<T, U, Cb>(env, cbinfo)
    .unwrap_or_else(|err| throw_error(env, err, "Error in Promise.then"))
}

#[inline]
fn handle_then_callback<T, U, Cb>(
  env: sys::napi_env,
  cbinfo: sys::napi_callback_info,
) -> Result<sys::napi_value>
where
  T: FromNapiValue,
  U: ToNapiValue,
  Cb: FnOnce(CallbackContext<T>) -> Result<U>,
{
  let mut callback_values = [ptr::null_mut()];
  let mut rust_cb = ptr::null_mut();
  check_status!(
    unsafe {
      sys::napi_get_cb_info(
        env,
        cbinfo,
        &mut 1,
        callback_values.as_mut_ptr(),
        ptr::null_mut(),
        &mut rust_cb,
      )
    },
    "Get callback info from then callback failed"
  )?;
  let then_value: T = unsafe { FromNapiValue::from_napi_value(env, callback_values[0]) }?;
  let cb: Box<(Cb, *mut bool)> = unsafe { Box::from_raw(rust_cb.cast()) };
  let executed = unsafe { Box::leak(Box::from_raw(cb.1)) };
  *executed = true;

  unsafe {
    U::to_napi_value(
      env,
      cb.0(CallbackContext {
        env: Env(env),
        value: then_value,
      })?,
    )
  }
}

unsafe extern "C" fn raw_promise_catch_callback<E, U, Cb>(
  env: sys::napi_env,
  cbinfo: sys::napi_callback_info,
) -> sys::napi_value
where
  E: FromNapiValue,
  U: ToNapiValue,
  Cb: FnOnce(CallbackContext<E>) -> Result<U>,
{
  handle_catch_callback::<E, U, Cb>(env, cbinfo)
    .unwrap_or_else(|err| throw_error(env, err, "Error in Promise.catch"))
}

#[inline(always)]
fn handle_catch_callback<E, U, Cb>(
  env: sys::napi_env,
  cbinfo: sys::napi_callback_info,
) -> Result<sys::napi_value>
where
  E: FromNapiValue,
  U: ToNapiValue,
  Cb: FnOnce(CallbackContext<E>) -> Result<U>,
{
  let mut callback_values = [ptr::null_mut(); 1];
  let mut rust_cb = ptr::null_mut();
  check_status!(
    unsafe {
      sys::napi_get_cb_info(
        env,
        cbinfo,
        &mut 1,
        callback_values.as_mut_ptr(),
        ptr::null_mut(),
        &mut rust_cb,
      )
    },
    "Get callback info from catch callback failed"
  )?;
  let catch_value: E = unsafe { FromNapiValue::from_napi_value(env, callback_values[0]) }?;
  let cb: Box<(Cb, *mut bool)> = unsafe { Box::from_raw(rust_cb.cast()) };

  let executed = unsafe { Box::leak(Box::from_raw(cb.1)) };
  *executed = true;

  unsafe {
    U::to_napi_value(
      env,
      cb.0(CallbackContext {
        env: Env(env),
        value: catch_value,
      })?,
    )
  }
}

unsafe extern "C" fn raw_promise_finally_callback<U, Cb>(
  env: sys::napi_env,
  cbinfo: sys::napi_callback_info,
) -> sys::napi_value
where
  U: ToNapiValue,
  Cb: FnOnce(Env) -> Result<U>,
{
  handle_finally_callback::<U, Cb>(env, cbinfo)
    .unwrap_or_else(|err| throw_error(env, err, "Error in Promise.finally"))
}

#[inline(always)]
fn handle_finally_callback<U, Cb>(
  env: sys::napi_env,
  cbinfo: sys::napi_callback_info,
) -> Result<sys::napi_value>
where
  U: ToNapiValue,
  Cb: FnOnce(Env) -> Result<U>,
{
  let mut rust_cb = ptr::null_mut();
  check_status!(
    unsafe {
      sys::napi_get_cb_info(
        env,
        cbinfo,
        &mut 0,
        ptr::null_mut(),
        ptr::null_mut(),
        &mut rust_cb,
      )
    },
    "Get callback info from finally callback failed"
  )?;
  let cb: Box<Cb> = unsafe { Box::from_raw(rust_cb.cast()) };

  unsafe { U::to_napi_value(env, cb(Env(env))?) }
}

pub struct CallbackContext<T> {
  pub env: Env,
  pub value: T,
}

impl<T: ToNapiValue> ToNapiValue for CallbackContext<T> {
  unsafe fn to_napi_value(env: napi_sys::napi_env, val: Self) -> Result<napi_sys::napi_value> {
    T::to_napi_value(env, val.value)
  }
}

#[inline(never)]
fn throw_error(env: sys::napi_env, err: Error, default_msg: &str) -> sys::napi_value {
  const GENERIC_FAILURE: &str = "GenericFailure\0";
  let code = if err.status.as_ref().is_empty() {
    GENERIC_FAILURE
  } else {
    err.status.as_ref()
  };
  let mut code_string = ptr::null_mut();
  let msg = if err.reason.is_empty() {
    default_msg
  } else {
    err.reason.as_ref()
  };
  let mut msg_string = ptr::null_mut();
  let mut err = ptr::null_mut();
  unsafe {
    sys::napi_create_string_latin1(
      env,
      code.as_ptr().cast(),
      code.len() as isize,
      &mut code_string,
    );
    sys::napi_create_string_utf8(
      env,
      msg.as_ptr().cast(),
      msg.len() as isize,
      &mut msg_string,
    );
    sys::napi_create_error(env, code_string, msg_string, &mut err);
    sys::napi_throw(env, err);
  };
  ptr::null_mut()
}

extern "C" fn promise_callback_finalizer<T, U, Cb>(
  _env: sys::napi_env,
  finalize_data: *mut c_void,
  finalize_hint: *mut c_void,
) where
  T: FromNapiValue,
  U: ToNapiValue,
  Cb: FnOnce(CallbackContext<T>) -> Result<U>,
{
  if !unsafe { *Box::from_raw(finalize_data.cast()) } {
    drop(unsafe { Box::from_raw(finalize_hint.cast::<Cb>()) });
  }
}
