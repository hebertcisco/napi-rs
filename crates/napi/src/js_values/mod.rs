#![allow(deprecated)]

use std::convert::TryFrom;
#[cfg(feature = "napi5")]
use std::ffi::c_void;
use std::ffi::CString;
use std::ptr;

use crate::{
  bindgen_runtime::{FromNapiValue, ToNapiValue, TypeName, ValidateNapiValue},
  check_status, sys, type_of, Callback, Error, Result, Status, ValueType,
};

#[cfg(feature = "serde-json")]
mod de;
#[cfg(feature = "serde-json")]
mod ser;

mod arraybuffer;
#[cfg(all(feature = "napi6", feature = "compat-mode"))]
mod bigint;
#[cfg(feature = "compat-mode")]
mod boolean;
#[cfg(feature = "compat-mode")]
mod buffer;
#[cfg(feature = "napi5")]
mod date;
#[cfg(feature = "napi4")]
mod deferred;
mod either;
mod escapable_handle_scope;
#[cfg(feature = "compat-mode")]
mod function;
mod global;
mod number;
mod object;
mod object_property;
mod string;
mod tagged_object;
#[cfg(feature = "compat-mode")]
mod undefined;
mod value;
mod value_ref;

pub use arraybuffer::*;
#[cfg(all(feature = "napi6", feature = "compat-mode"))]
pub use bigint::JsBigInt;
#[cfg(feature = "compat-mode")]
pub use boolean::JsBoolean;
#[cfg(feature = "compat-mode")]
pub use buffer::*;
#[cfg(feature = "napi5")]
pub use date::*;
#[cfg(feature = "serde-json")]
pub use de::De;
#[cfg(feature = "napi4")]
pub use deferred::*;
pub use either::Either;
pub use escapable_handle_scope::EscapableHandleScope;
#[cfg(feature = "compat-mode")]
pub use function::JsFunction;
pub use global::*;
pub use number::JsNumber;
pub use object::*;
pub use object_property::*;
#[cfg(feature = "serde-json")]
pub use ser::Ser;
pub use string::*;
pub(crate) use tagged_object::TaggedObject;
#[cfg(feature = "compat-mode")]
pub use undefined::JsUndefined;
pub(crate) use value::Value;
pub use value_ref::*;

// Value types

#[cfg(feature = "compat-mode")]
#[deprecated(
  since = "3.0.0",
  note = "Please use `napi::bindgen_prelude::Null` instead"
)]
#[derive(Clone, Copy)]
pub struct JsNull(pub(crate) Value);

#[cfg(feature = "compat-mode")]
impl TypeName for JsNull {
  fn type_name() -> &'static str {
    "null"
  }

  fn value_type() -> ValueType {
    ValueType::Null
  }
}

#[cfg(feature = "compat-mode")]
impl ValidateNapiValue for JsNull {}

#[derive(Clone, Copy)]
pub struct JsSymbol<'env>(
  pub(crate) Value,
  pub(crate) std::marker::PhantomData<&'env ()>,
);

impl TypeName for JsSymbol<'_> {
  fn type_name() -> &'static str {
    "symbol"
  }

  fn value_type() -> ValueType {
    ValueType::Symbol
  }
}

impl<'env> JsValue<'env> for JsSymbol<'env> {
  fn value(&self) -> Value {
    self.0
  }
}

impl FromNapiValue for JsSymbol<'_> {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> Result<Self> {
    Ok(JsSymbol(
      Value {
        env,
        value: napi_val,
        value_type: ValueType::Symbol,
      },
      std::marker::PhantomData,
    ))
  }
}

impl ValidateNapiValue for JsSymbol<'_> {}

#[cfg(feature = "compat-mode")]
#[deprecated(
  since = "3.0.0",
  note = "Please use `napi::bindgen_prelude::External` instead"
)]
pub struct JsExternal(pub(crate) Value);

#[cfg(feature = "compat-mode")]
impl TypeName for JsExternal {
  fn type_name() -> &'static str {
    "external"
  }

  fn value_type() -> ValueType {
    ValueType::External
  }
}

#[cfg(feature = "compat-mode")]
impl ValidateNapiValue for JsExternal {}

pub trait JsValue<'env>: Sized {
  fn value(&self) -> Value;

  /// Convert the value to an unknown
  fn to_unknown(&self) -> Unknown<'env> {
    Unknown(
      Value {
        env: self.value().env,
        value: self.value().value,
        value_type: ValueType::Unknown,
      },
      std::marker::PhantomData,
    )
  }

  /// Coerce the value to a boolean
  fn coerce_to_bool(&self) -> Result<bool> {
    let mut new_raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_coerce_to_bool(env, self.value().value, &mut new_raw_value)
    })?;
    unsafe { bool::from_napi_value(env, new_raw_value) }
  }

  fn coerce_to_number(&self) -> Result<JsNumber> {
    let mut new_raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_coerce_to_number(env, self.value().value, &mut new_raw_value)
    })?;
    Ok(JsNumber(
      Value {
        env,
        value: new_raw_value,
        value_type: ValueType::Number,
      },
      std::marker::PhantomData,
    ))
  }

  fn coerce_to_string(&self) -> Result<JsString> {
    let mut new_raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_coerce_to_string(env, self.value().value, &mut new_raw_value)
    })?;
    Ok(JsString(
      Value {
        env,
        value: new_raw_value,
        value_type: ValueType::String,
      },
      std::marker::PhantomData,
    ))
  }

  fn coerce_to_object(self) -> Result<JsObject> {
    let mut new_raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_coerce_to_object(env, self.value().value, &mut new_raw_value)
    })?;
    Ok(JsObject(Value {
      env,
      value: new_raw_value,
      value_type: ValueType::Object,
    }))
  }

  #[cfg(feature = "napi5")]
  fn is_date(&self) -> Result<bool> {
    let mut is_date = true;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_is_date(env, self.value().value, &mut is_date) })?;
    Ok(is_date)
  }

  fn is_promise(&self) -> Result<bool> {
    let mut is_promise = true;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_is_promise(env, self.value().value, &mut is_promise) })?;
    Ok(is_promise)
  }

  fn is_error(&self) -> Result<bool> {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_is_error(env, self.value().value, &mut result) })?;
    Ok(result)
  }

  fn is_typedarray(&self) -> Result<bool> {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_is_typedarray(env, self.value().value, &mut result) })?;
    Ok(result)
  }

  fn is_dataview(&self) -> Result<bool> {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_is_dataview(env, self.value().value, &mut result) })?;
    Ok(result)
  }

  fn is_array(&self) -> Result<bool> {
    let mut is_array = false;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_is_array(env, self.value().value, &mut is_array) })?;
    Ok(is_array)
  }

  fn is_buffer(&self) -> Result<bool> {
    let mut is_buffer = false;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_is_buffer(env, self.value().value, &mut is_buffer) })?;
    Ok(is_buffer)
  }

  fn is_arraybuffer(&self) -> Result<bool> {
    let mut is_buffer = false;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_is_arraybuffer(env, self.value().value, &mut is_buffer) })?;
    Ok(is_buffer)
  }

  fn instanceof<Constructor>(&self, constructor: Constructor) -> Result<bool>
  where
    Constructor: NapiRaw,
  {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_instanceof(env, self.value().value, constructor.raw(), &mut result)
    })?;
    Ok(result)
  }
}

pub trait JsObjectValue<'env>: JsValue<'env> {
  fn set_property<K, V>(&mut self, key: K, value: V) -> Result<()>
  where
    K: NapiRaw,
    V: NapiRaw,
  {
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_set_property(env, self.value().value, key.raw(), value.raw())
    })
  }

  fn get_property<K, T>(&self, key: K) -> Result<T>
  where
    K: NapiRaw,
    T: NapiValue,
  {
    let mut raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_get_property(env, self.value().value, key.raw(), &mut raw_value)
    })?;
    unsafe { T::from_raw(env, raw_value) }
  }

  fn get_property_unchecked<K, T>(&self, key: K) -> Result<T>
  where
    K: NapiRaw,
    T: NapiValue,
  {
    let mut raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_get_property(env, self.value().value, key.raw(), &mut raw_value)
    })?;
    Ok(unsafe { T::from_raw_unchecked(env, raw_value) })
  }

  fn set_named_property<T>(&mut self, name: &str, value: T) -> Result<()>
  where
    T: ToNapiValue,
  {
    let key = CString::new(name)?;
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_set_named_property(
        env,
        self.value().value,
        key.as_ptr(),
        T::to_napi_value(env, value)?,
      )
    })
  }

  fn create_named_method(&mut self, name: &str, function: Callback) -> Result<()> {
    let mut js_function = ptr::null_mut();
    let len = name.len();
    let name = CString::new(name)?;
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_create_function(
        env,
        name.as_ptr(),
        len as isize,
        Some(function),
        ptr::null_mut(),
        &mut js_function,
      )
    })?;
    check_status!(
      unsafe { sys::napi_set_named_property(env, self.value().value, name.as_ptr(), js_function) },
      "create_named_method error"
    )
  }

  fn get_named_property<T>(&self, name: &str) -> Result<T>
  where
    T: FromNapiValue + ValidateNapiValue,
  {
    let key = CString::new(name)?;
    let mut raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(
      unsafe {
        sys::napi_get_named_property(env, self.value().value, key.as_ptr(), &mut raw_value)
      },
      "get_named_property error"
    )?;
    unsafe { <T as ValidateNapiValue>::validate(env, raw_value) }.map_err(|mut err| {
      err.reason = format!("Object property '{name}' type mismatch. {}", err.reason);
      err
    })?;
    unsafe { <T as FromNapiValue>::from_napi_value(env, raw_value) }
  }

  fn get_named_property_unchecked<T>(&self, name: &str) -> Result<T>
  where
    T: FromNapiValue,
  {
    let key = CString::new(name)?;
    let mut raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(
      unsafe {
        sys::napi_get_named_property(env, self.value().value, key.as_ptr(), &mut raw_value)
      },
      "get_named_property_unchecked error"
    )?;
    unsafe { <T as FromNapiValue>::from_napi_value(env, raw_value) }
  }

  fn has_named_property<N: AsRef<str>>(&self, name: N) -> Result<bool> {
    let mut result = false;
    let key = CString::new(name.as_ref())?;
    let env = self.value().env;
    check_status!(
      unsafe { sys::napi_has_named_property(env, self.value().value, key.as_ptr(), &mut result) },
      "napi_has_named_property error"
    )?;
    Ok(result)
  }

  fn delete_property<S>(&mut self, name: S) -> Result<bool>
  where
    S: NapiRaw,
  {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_delete_property(env, self.value().value, name.raw(), &mut result)
    })?;
    Ok(result)
  }

  fn delete_named_property(&mut self, name: &str) -> Result<bool> {
    let mut result = false;
    let mut js_key = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_create_string_utf8(env, name.as_ptr().cast(), name.len() as isize, &mut js_key)
    })?;
    check_status!(unsafe {
      sys::napi_delete_property(env, self.value().value, js_key, &mut result)
    })?;
    Ok(result)
  }

  fn has_own_property(&self, key: &str) -> Result<bool> {
    let mut result = false;
    let mut js_key = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_create_string_utf8(env, key.as_ptr().cast(), key.len() as isize, &mut js_key)
    })?;
    check_status!(unsafe {
      sys::napi_has_own_property(env, self.value().value, js_key, &mut result)
    })?;
    Ok(result)
  }

  fn has_own_property_js<K>(&self, key: K) -> Result<bool>
  where
    K: NapiRaw,
  {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_has_own_property(env, self.value().value, key.raw(), &mut result)
    })?;
    Ok(result)
  }

  fn has_property(&self, name: &str) -> Result<bool> {
    let mut js_key = ptr::null_mut();
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_create_string_utf8(env, name.as_ptr().cast(), name.len() as isize, &mut js_key)
    })?;
    check_status!(unsafe { sys::napi_has_property(env, self.value().value, js_key, &mut result) })?;
    Ok(result)
  }

  fn has_property_js<K>(&self, name: K) -> Result<bool>
  where
    K: NapiRaw,
  {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_has_property(env, self.value().value, name.raw(), &mut result)
    })?;
    Ok(result)
  }

  fn get_property_names(&self) -> Result<JsObject> {
    let mut raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_get_property_names(env, self.value().value, &mut raw_value)
    })?;
    Ok(unsafe { JsObject::from_raw_unchecked(env, raw_value) })
  }

  /// <https://nodejs.org/api/n-api.html#n_api_napi_get_all_property_names>
  /// return `Array` of property names
  #[cfg(feature = "napi6")]
  fn get_all_property_names(
    &self,
    mode: KeyCollectionMode,
    filter: KeyFilter,
    conversion: KeyConversion,
  ) -> Result<JsObject> {
    let mut properties_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_get_all_property_names(
        env,
        self.value().value,
        mode.into(),
        filter.into(),
        conversion.into(),
        &mut properties_value,
      )
    })?;
    Ok(unsafe { JsObject::from_raw_unchecked(env, properties_value) })
  }

  /// This returns the equivalent of `Object.getPrototypeOf` (which is not the same as the function's prototype property).
  fn get_prototype<T>(&self) -> Result<T>
  where
    T: NapiValue,
  {
    let mut result = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe { sys::napi_get_prototype(env, self.value().value, &mut result) })?;
    unsafe { T::from_raw(env, result) }
  }

  fn get_prototype_unchecked<T>(&self) -> Result<T>
  where
    T: NapiValue,
  {
    let mut result = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe { sys::napi_get_prototype(env, self.value().value, &mut result) })?;
    Ok(unsafe { T::from_raw_unchecked(env, result) })
  }

  fn set_element<T>(&mut self, index: u32, value: T) -> Result<()>
  where
    T: NapiRaw,
  {
    let env = self.value().env;
    check_status!(unsafe { sys::napi_set_element(env, self.value().value, index, value.raw()) })
  }

  fn has_element(&self, index: u32) -> Result<bool> {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_has_element(env, self.value().value, index, &mut result) })?;
    Ok(result)
  }

  fn delete_element(&mut self, index: u32) -> Result<bool> {
    let mut result = false;
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_delete_element(env, self.value().value, index, &mut result)
    })?;
    Ok(result)
  }

  fn get_element<T>(&self, index: u32) -> Result<T>
  where
    T: NapiValue,
  {
    let mut raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_get_element(env, self.value().value, index, &mut raw_value)
    })?;
    unsafe { T::from_raw(env, raw_value) }
  }

  fn get_element_unchecked<T>(&self, index: u32) -> Result<T>
  where
    T: NapiValue,
  {
    let mut raw_value = ptr::null_mut();
    let env = self.value().env;
    check_status!(unsafe {
      sys::napi_get_element(env, self.value().value, index, &mut raw_value)
    })?;
    Ok(unsafe { T::from_raw_unchecked(env, raw_value) })
  }

  /// This method allows the efficient definition of multiple properties on a given object.
  fn define_properties(&mut self, properties: &[Property]) -> Result<()> {
    let properties_iter = properties.iter().map(|property| property.raw());
    let env = self.value().env;
    #[cfg(feature = "napi5")]
    {
      let mut closures = properties_iter
        .clone()
        .map(|p| p.data)
        .filter(|data| !data.is_null())
        .collect::<Vec<*mut std::ffi::c_void>>();
      let len = Box::into_raw(Box::new(closures.len()));
      check_status!(unsafe {
        sys::napi_add_finalizer(
          env,
          self.value().value,
          closures.as_mut_ptr().cast(),
          Some(finalize_closures),
          len.cast(),
          ptr::null_mut(),
        )
      })?;
      std::mem::forget(closures);
    }
    check_status!(unsafe {
      sys::napi_define_properties(
        env,
        self.value().value,
        properties.len(),
        properties_iter
          .collect::<Vec<sys::napi_property_descriptor>>()
          .as_ptr(),
      )
    })
  }

  /// Perform `is_array` check before get the length
  /// if `Object` is not array, `ArrayExpected` error returned
  fn get_array_length(&self) -> Result<u32> {
    if !(self.is_array()?) {
      return Err(Error::new(
        Status::ArrayExpected,
        "Object is not array".to_owned(),
      ));
    }
    self.get_array_length_unchecked()
  }

  /// use this API if you can ensure this `Object` is `Array`
  fn get_array_length_unchecked(&self) -> Result<u32> {
    let mut length: u32 = 0;
    let env = self.value().env;
    check_status!(unsafe { sys::napi_get_array_length(env, self.value().value, &mut length) })?;
    Ok(length)
  }

  #[cfg(feature = "napi8")]
  fn freeze(&mut self) -> Result<()> {
    let env = self.value().env;
    check_status!(unsafe { sys::napi_object_freeze(env, self.value().value) })
  }

  #[cfg(feature = "napi8")]
  fn seal(&mut self) -> Result<()> {
    let env = self.value().env;
    check_status!(unsafe { sys::napi_object_seal(env, self.value().value) })
  }
}

impl<'env, T: JsValue<'env>> NapiRaw for T {
  unsafe fn raw(&self) -> sys::napi_value {
    self.value().value
  }
}

macro_rules! impl_napi_value_trait {
  ($js_value:ident, $value_type:ident) => {
    impl NapiValue for $js_value {
      unsafe fn from_raw(env: sys::napi_env, value: sys::napi_value) -> Result<$js_value> {
        let value_type = type_of!(env, value)?;
        if value_type != $value_type {
          Err(Error::new(
            Status::InvalidArg,
            format!("expect {:?}, got: {:?}", $value_type, value_type),
          ))
        } else {
          Ok($js_value(Value {
            env,
            value,
            value_type: $value_type,
          }))
        }
      }

      unsafe fn from_raw_unchecked(env: sys::napi_env, value: sys::napi_value) -> $js_value {
        $js_value(Value {
          env,
          value,
          value_type: $value_type,
        })
      }
    }

    impl NapiRaw for $js_value {
      unsafe fn raw(&self) -> sys::napi_value {
        self.0.value
      }
    }

    impl<'env> NapiRaw for &'env $js_value {
      unsafe fn raw(&self) -> sys::napi_value {
        self.0.value
      }
    }

    impl TryFrom<Unknown<'_>> for $js_value {
      type Error = Error;
      fn try_from(value: Unknown) -> Result<$js_value> {
        unsafe { $js_value::from_raw(value.0.env, value.0.value) }
      }
    }
  };
}

macro_rules! impl_js_value_methods {
  ($js_value:ident) => {
    impl $js_value {
      pub fn into_unknown<'env>(self) -> Unknown<'env> {
        unsafe { Unknown::from_raw_unchecked(self.0.env, self.0.value) }
      }

      #[cfg(feature = "compat-mode")]
      pub fn coerce_to_bool(self) -> Result<JsBoolean> {
        let mut new_raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_coerce_to_bool(self.0.env, self.0.value, &mut new_raw_value)
        })?;
        Ok(JsBoolean(Value {
          env: self.0.env,
          value: new_raw_value,
          value_type: ValueType::Boolean,
        }))
      }

      pub fn coerce_to_number<'env>(self) -> Result<JsNumber<'env>> {
        let mut new_raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_coerce_to_number(self.0.env, self.0.value, &mut new_raw_value)
        })?;
        Ok(JsNumber(
          Value {
            env: self.0.env,
            value: new_raw_value,
            value_type: ValueType::Number,
          },
          std::marker::PhantomData,
        ))
      }

      pub fn coerce_to_string<'env>(self) -> Result<JsString<'env>> {
        let mut new_raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_coerce_to_string(self.0.env, self.0.value, &mut new_raw_value)
        })?;
        Ok(JsString(
          Value {
            env: self.0.env,
            value: new_raw_value,
            value_type: ValueType::String,
          },
          std::marker::PhantomData,
        ))
      }

      pub fn coerce_to_object(self) -> Result<JsObject> {
        let mut new_raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_coerce_to_object(self.0.env, self.0.value, &mut new_raw_value)
        })?;
        Ok(JsObject(Value {
          env: self.0.env,
          value: new_raw_value,
          value_type: ValueType::Object,
        }))
      }

      #[cfg(feature = "napi5")]
      pub fn is_date(&self) -> Result<bool> {
        let mut is_date = true;
        check_status!(unsafe { sys::napi_is_date(self.0.env, self.0.value, &mut is_date) })?;
        Ok(is_date)
      }

      pub fn is_promise(&self) -> Result<bool> {
        let mut is_promise = true;
        check_status!(unsafe { sys::napi_is_promise(self.0.env, self.0.value, &mut is_promise) })?;
        Ok(is_promise)
      }

      pub fn is_error(&self) -> Result<bool> {
        let mut result = false;
        check_status!(unsafe { sys::napi_is_error(self.0.env, self.0.value, &mut result) })?;
        Ok(result)
      }

      pub fn is_typedarray(&self) -> Result<bool> {
        let mut result = false;
        check_status!(unsafe { sys::napi_is_typedarray(self.0.env, self.0.value, &mut result) })?;
        Ok(result)
      }

      pub fn is_dataview(&self) -> Result<bool> {
        let mut result = false;
        check_status!(unsafe { sys::napi_is_dataview(self.0.env, self.0.value, &mut result) })?;
        Ok(result)
      }

      pub fn is_array(&self) -> Result<bool> {
        let mut is_array = false;
        check_status!(unsafe { sys::napi_is_array(self.0.env, self.0.value, &mut is_array) })?;
        Ok(is_array)
      }

      pub fn is_buffer(&self) -> Result<bool> {
        let mut is_buffer = false;
        check_status!(unsafe { sys::napi_is_buffer(self.0.env, self.0.value, &mut is_buffer) })?;
        Ok(is_buffer)
      }

      pub fn is_arraybuffer(&self) -> Result<bool> {
        let mut is_buffer = false;
        check_status!(unsafe {
          sys::napi_is_arraybuffer(self.0.env, self.0.value, &mut is_buffer)
        })?;
        Ok(is_buffer)
      }

      pub fn instanceof<Constructor>(&self, constructor: Constructor) -> Result<bool>
      where
        Constructor: NapiRaw,
      {
        let mut result = false;
        check_status!(unsafe {
          sys::napi_instanceof(self.0.env, self.0.value, constructor.raw(), &mut result)
        })?;
        Ok(result)
      }
    }
  };
}

macro_rules! impl_object_methods {
  ($js_value:ident) => {
    impl $js_value {
      pub fn set_property<K, V>(&mut self, key: K, value: V) -> Result<()>
      where
        K: NapiRaw,
        V: NapiRaw,
      {
        check_status!(unsafe {
          sys::napi_set_property(self.0.env, self.0.value, key.raw(), value.raw())
        })
      }

      pub fn get_property<K, T>(&self, key: K) -> Result<T>
      where
        K: NapiRaw,
        T: FromNapiValue,
      {
        let mut raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_get_property(self.0.env, self.0.value, key.raw(), &mut raw_value)
        })?;
        unsafe { T::from_napi_value(self.0.env, raw_value) }
      }

      pub fn get_property_unchecked<K, T>(&self, key: K) -> Result<T>
      where
        K: NapiRaw,
        T: NapiValue,
      {
        let mut raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_get_property(self.0.env, self.0.value, key.raw(), &mut raw_value)
        })?;
        Ok(unsafe { T::from_raw_unchecked(self.0.env, raw_value) })
      }

      pub fn set_named_property<T>(&mut self, name: &str, value: T) -> Result<()>
      where
        T: ToNapiValue,
      {
        let key = CString::new(name)?;
        check_status!(unsafe {
          sys::napi_set_named_property(
            self.0.env,
            self.0.value,
            key.as_ptr(),
            T::to_napi_value(self.0.env, value)?,
          )
        })
      }

      pub fn create_named_method(&mut self, name: &str, function: Callback) -> Result<()> {
        let mut js_function = ptr::null_mut();
        let len = name.len();
        let name = CString::new(name)?;
        check_status!(unsafe {
          sys::napi_create_function(
            self.0.env,
            name.as_ptr(),
            len as isize,
            Some(function),
            ptr::null_mut(),
            &mut js_function,
          )
        })?;
        check_status!(
          unsafe {
            sys::napi_set_named_property(self.0.env, self.0.value, name.as_ptr(), js_function)
          },
          "create_named_method error"
        )
      }

      pub fn get_named_property<T>(&self, name: &str) -> Result<T>
      where
        T: FromNapiValue + ValidateNapiValue,
      {
        let key = CString::new(name)?;
        let mut raw_value = ptr::null_mut();
        check_status!(
          unsafe {
            sys::napi_get_named_property(self.0.env, self.0.value, key.as_ptr(), &mut raw_value)
          },
          "get_named_property error"
        )?;
        unsafe { <T as ValidateNapiValue>::validate(self.0.env, raw_value) }.map_err(
          |mut err| {
            err.reason = format!("Object property '{name}' type mismatch. {}", err.reason);
            err
          },
        )?;
        unsafe { <T as FromNapiValue>::from_napi_value(self.0.env, raw_value) }
      }

      pub fn get_named_property_unchecked<T>(&self, name: &str) -> Result<T>
      where
        T: FromNapiValue,
      {
        let key = CString::new(name)?;
        let mut raw_value = ptr::null_mut();
        check_status!(
          unsafe {
            sys::napi_get_named_property(self.0.env, self.0.value, key.as_ptr(), &mut raw_value)
          },
          "get_named_property_unchecked error"
        )?;
        unsafe { <T as FromNapiValue>::from_napi_value(self.0.env, raw_value) }
      }

      pub fn has_named_property<N: AsRef<str>>(&self, name: N) -> Result<bool> {
        let mut result = false;
        let key = CString::new(name.as_ref())?;
        check_status!(
          unsafe {
            sys::napi_has_named_property(self.0.env, self.0.value, key.as_ptr(), &mut result)
          },
          "napi_has_named_property error"
        )?;
        Ok(result)
      }

      pub fn delete_property<S>(&mut self, name: S) -> Result<bool>
      where
        S: NapiRaw,
      {
        let mut result = false;
        check_status!(unsafe {
          sys::napi_delete_property(self.0.env, self.0.value, name.raw(), &mut result)
        })?;
        Ok(result)
      }

      pub fn delete_named_property(&mut self, name: &str) -> Result<bool> {
        let mut result = false;
        let mut js_key = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_create_string_utf8(
            self.0.env,
            name.as_ptr().cast(),
            name.len() as isize,
            &mut js_key,
          )
        })?;
        check_status!(unsafe {
          sys::napi_delete_property(self.0.env, self.0.value, js_key, &mut result)
        })?;
        Ok(result)
      }

      pub fn has_own_property(&self, key: &str) -> Result<bool> {
        let mut result = false;
        let mut js_key = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_create_string_utf8(
            self.0.env,
            key.as_ptr().cast(),
            key.len() as isize,
            &mut js_key,
          )
        })?;
        check_status!(unsafe {
          sys::napi_has_own_property(self.0.env, self.0.value, js_key, &mut result)
        })?;
        Ok(result)
      }

      pub fn has_own_property_js<K>(&self, key: K) -> Result<bool>
      where
        K: NapiRaw,
      {
        let mut result = false;
        check_status!(unsafe {
          sys::napi_has_own_property(self.0.env, self.0.value, key.raw(), &mut result)
        })?;
        Ok(result)
      }

      pub fn has_property(&self, name: &str) -> Result<bool> {
        let mut js_key = ptr::null_mut();
        let mut result = false;
        check_status!(unsafe {
          sys::napi_create_string_utf8(
            self.0.env,
            name.as_ptr().cast(),
            name.len() as isize,
            &mut js_key,
          )
        })?;
        check_status!(unsafe {
          sys::napi_has_property(self.0.env, self.0.value, js_key, &mut result)
        })?;
        Ok(result)
      }

      pub fn has_property_js<K>(&self, name: K) -> Result<bool>
      where
        K: NapiRaw,
      {
        let mut result = false;
        check_status!(unsafe {
          sys::napi_has_property(self.0.env, self.0.value, name.raw(), &mut result)
        })?;
        Ok(result)
      }

      pub fn get_property_names(&self) -> Result<JsObject> {
        let mut raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_get_property_names(self.0.env, self.0.value, &mut raw_value)
        })?;
        Ok(unsafe { JsObject::from_raw_unchecked(self.0.env, raw_value) })
      }

      /// <https://nodejs.org/api/n-api.html#n_api_napi_get_all_property_names>
      /// return `Array` of property names
      #[cfg(feature = "napi6")]
      pub fn get_all_property_names(
        &self,
        mode: KeyCollectionMode,
        filter: KeyFilter,
        conversion: KeyConversion,
      ) -> Result<JsObject> {
        let mut properties_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_get_all_property_names(
            self.0.env,
            self.0.value,
            mode.into(),
            filter.into(),
            conversion.into(),
            &mut properties_value,
          )
        })?;
        Ok(unsafe { JsObject::from_raw_unchecked(self.0.env, properties_value) })
      }

      /// This returns the equivalent of `Object.getPrototypeOf` (which is not the same as the function's prototype property).
      pub fn get_prototype<T>(&self) -> Result<T>
      where
        T: NapiValue,
      {
        let mut result = ptr::null_mut();
        check_status!(unsafe { sys::napi_get_prototype(self.0.env, self.0.value, &mut result) })?;
        unsafe { T::from_raw(self.0.env, result) }
      }

      pub fn get_prototype_unchecked<T>(&self) -> Result<T>
      where
        T: NapiValue,
      {
        let mut result = ptr::null_mut();
        check_status!(unsafe { sys::napi_get_prototype(self.0.env, self.0.value, &mut result) })?;
        Ok(unsafe { T::from_raw_unchecked(self.0.env, result) })
      }

      pub fn set_element<T>(&mut self, index: u32, value: T) -> Result<()>
      where
        T: NapiRaw,
      {
        check_status!(unsafe {
          sys::napi_set_element(self.0.env, self.0.value, index, value.raw())
        })
      }

      pub fn has_element(&self, index: u32) -> Result<bool> {
        let mut result = false;
        check_status!(unsafe {
          sys::napi_has_element(self.0.env, self.0.value, index, &mut result)
        })?;
        Ok(result)
      }

      pub fn delete_element(&mut self, index: u32) -> Result<bool> {
        let mut result = false;
        check_status!(unsafe {
          sys::napi_delete_element(self.0.env, self.0.value, index, &mut result)
        })?;
        Ok(result)
      }

      pub fn get_element<T>(&self, index: u32) -> Result<T>
      where
        T: FromNapiValue,
      {
        let mut raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_get_element(self.0.env, self.0.value, index, &mut raw_value)
        })?;
        unsafe { T::from_napi_value(self.0.env, raw_value) }
      }

      pub fn get_element_unchecked<T>(&self, index: u32) -> Result<T>
      where
        T: NapiValue,
      {
        let mut raw_value = ptr::null_mut();
        check_status!(unsafe {
          sys::napi_get_element(self.0.env, self.0.value, index, &mut raw_value)
        })?;
        Ok(unsafe { T::from_raw_unchecked(self.0.env, raw_value) })
      }

      /// This method allows the efficient definition of multiple properties on a given object.
      pub fn define_properties(&mut self, properties: &[Property]) -> Result<()> {
        let properties_iter = properties.iter().map(|property| property.raw());
        #[cfg(feature = "napi5")]
        {
          let mut closures = properties_iter
            .clone()
            .map(|p| p.data)
            .filter(|data| !data.is_null())
            .collect::<Vec<*mut std::ffi::c_void>>();
          let len = Box::into_raw(Box::new(closures.len()));
          check_status!(unsafe {
            sys::napi_add_finalizer(
              self.0.env,
              self.0.value,
              closures.as_mut_ptr().cast(),
              Some(finalize_closures),
              len.cast(),
              ptr::null_mut(),
            )
          })?;
          std::mem::forget(closures);
        }
        check_status!(unsafe {
          sys::napi_define_properties(
            self.0.env,
            self.0.value,
            properties.len(),
            properties_iter
              .collect::<Vec<sys::napi_property_descriptor>>()
              .as_ptr(),
          )
        })
      }

      /// Perform `is_array` check before get the length
      /// if `Object` is not array, `ArrayExpected` error returned
      pub fn get_array_length(&self) -> Result<u32> {
        if self.is_array()? != true {
          return Err(Error::new(
            Status::ArrayExpected,
            "Object is not array".to_owned(),
          ));
        }
        self.get_array_length_unchecked()
      }

      /// use this API if you can ensure this `Object` is `Array`
      pub fn get_array_length_unchecked(&self) -> Result<u32> {
        let mut length: u32 = 0;
        check_status!(unsafe {
          sys::napi_get_array_length(self.0.env, self.0.value, &mut length)
        })?;
        Ok(length)
      }

      #[cfg(feature = "napi8")]
      pub fn freeze(&mut self) -> Result<()> {
        check_status!(unsafe { sys::napi_object_freeze(self.0.env, self.0.value) })
      }

      #[cfg(feature = "napi8")]
      pub fn seal(&mut self) -> Result<()> {
        check_status!(unsafe { sys::napi_object_seal(self.0.env, self.0.value) })
      }
    }
  };
}

pub trait NapiRaw {
  #[allow(clippy::missing_safety_doc)]
  unsafe fn raw(&self) -> sys::napi_value;
}

pub trait NapiValue: Sized + NapiRaw {
  #[allow(clippy::missing_safety_doc)]
  unsafe fn from_raw(env: sys::napi_env, value: sys::napi_value) -> Result<Self>;

  #[allow(clippy::missing_safety_doc)]
  unsafe fn from_raw_unchecked(env: sys::napi_env, value: sys::napi_value) -> Self;
}

#[cfg(feature = "compat-mode")]
impl_js_value_methods!(JsUndefined);
#[cfg(feature = "compat-mode")]
impl_js_value_methods!(JsNull);
#[cfg(feature = "compat-mode")]
impl_js_value_methods!(JsBoolean);
#[cfg(feature = "compat-mode")]
impl_js_value_methods!(JsBuffer);
impl_js_value_methods!(JsArrayBuffer);
impl_js_value_methods!(JsTypedArray);
impl_js_value_methods!(JsDataView);
impl_js_value_methods!(JsObject);
#[cfg(feature = "napi5")]
impl_js_value_methods!(JsDate);
#[cfg(feature = "compat-mode")]
impl_js_value_methods!(JsFunction);
#[cfg(feature = "compat-mode")]
impl_js_value_methods!(JsExternal);

impl_object_methods!(JsObject);
#[cfg(feature = "compat-mode")]
impl_object_methods!(JsBuffer);
impl_object_methods!(JsArrayBuffer);
impl_object_methods!(JsTypedArray);
impl_object_methods!(JsDataView);

use ValueType::*;

#[cfg(feature = "compat-mode")]
impl_napi_value_trait!(JsUndefined, Undefined);
#[cfg(feature = "compat-mode")]
impl_napi_value_trait!(JsNull, Null);
#[cfg(feature = "compat-mode")]
impl_napi_value_trait!(JsBoolean, Boolean);
#[cfg(feature = "compat-mode")]
impl_napi_value_trait!(JsBuffer, Object);
impl_napi_value_trait!(JsArrayBuffer, Object);
impl_napi_value_trait!(JsTypedArray, Object);
impl_napi_value_trait!(JsDataView, Object);
impl_napi_value_trait!(JsObject, Object);
#[cfg(feature = "napi5")]
impl_napi_value_trait!(JsDate, Object);
#[cfg(feature = "compat-mode")]
impl_napi_value_trait!(JsFunction, Function);
#[cfg(feature = "compat-mode")]
impl_napi_value_trait!(JsExternal, External);

/// Represents a raw JavaScript value
pub struct Unknown<'env>(
  pub(crate) Value,
  pub(crate) std::marker::PhantomData<&'env ()>,
);

impl<'env> JsValue<'env> for Unknown<'env> {
  fn value(&self) -> Value {
    self.0
  }
}

impl TypeName for Unknown<'_> {
  fn type_name() -> &'static str {
    "unknown"
  }

  fn value_type() -> ValueType {
    ValueType::Unknown
  }
}

impl ValidateNapiValue for Unknown<'_> {
  unsafe fn validate(
    _env: napi_sys::napi_env,
    _napi_val: napi_sys::napi_value,
  ) -> Result<napi_sys::napi_value> {
    Ok(ptr::null_mut())
  }
}

impl FromNapiValue for Unknown<'_> {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> Result<Self> {
    Ok(Unknown(
      Value {
        env,
        value: napi_val,
        value_type: ValueType::Unknown,
      },
      std::marker::PhantomData,
    ))
  }
}

impl Unknown<'_> {
  pub fn get_type(&self) -> Result<ValueType> {
    type_of!(self.0.env, self.0.value)
  }

  /// # Safety
  ///
  /// This function should be called after `JsUnknown::get_type`
  ///
  /// And the `V` must be match with the return value of `get_type`
  pub unsafe fn cast<V>(&self) -> V
  where
    V: NapiValue,
  {
    unsafe { V::from_raw_unchecked(self.0.env, self.0.value) }
  }

  /// # Safety
  ///
  /// JsUnknown doesn't have a type
  pub unsafe fn from_raw_unchecked(env: sys::napi_env, value: sys::napi_value) -> Self {
    Unknown(
      Value {
        env,
        value,
        value_type: ValueType::Unknown,
      },
      std::marker::PhantomData,
    )
  }
}

#[cfg(feature = "napi5")]
unsafe extern "C" fn finalize_closures(_env: sys::napi_env, data: *mut c_void, len: *mut c_void) {
  let length: usize = *unsafe { Box::from_raw(len.cast()) };
  let closures: Vec<*mut PropertyClosures> =
    unsafe { Vec::from_raw_parts(data.cast(), length, length) };
  for closure in closures.into_iter() {
    drop(unsafe { Box::from_raw(closure) });
  }
}
