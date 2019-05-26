use std::{fmt, result};
use std::ops::Deref;

use serde_json::Value;

use filter::value_filter::*;
use parser::parser::*;
use ref_value;
use ref_value::model::*;
use select::path_map::PathMap;
use std::sync::Arc;
use std::cell::RefCell;

/// Utility. Functions like jsonpath::selector or jsonpath::compile are also implemented using this structure.
///
/// ```rust
/// extern crate jsonpath_lib as jsonpath;
/// extern crate serde;
/// extern crate serde_json;
///
/// use serde::{Deserialize, Serialize};
/// use serde_json::Value;
///
/// use jsonpath::Selector;
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Person {
///     name: String,
///     age: Option<u8>,
///     phone: String,
/// }
///
/// fn input_str() -> &'static str {
///     r#"[
///         {
///             "name": "이름1",
///             "age": 40,
///             "phone": "+33 12341234"
///         },
///         {
///             "name": "이름2",
///             "age": 42,
///             "phone": "++44 12341234"
///         }
///     ]"#
/// }
///
/// fn input_json() -> Value {
///     serde_json::from_str(input_str()).unwrap()
/// }
///
/// fn input_person() -> Vec<Person> {
///     serde_json::from_str(input_str()).unwrap()
/// }
///
///
/// let mut selector = Selector::new();
///
/// let result = selector
///     .path("$..[?(@.age > 40)]").unwrap()
///     .value_from_str(input_str()).unwrap()
///     .select_as_value().unwrap();
/// assert_eq!(input_json()[1], result[0]);
///
/// let result = selector.select_as_str().unwrap();
/// assert_eq!(serde_json::to_string(&vec![&input_json()[1].clone()]).unwrap(), result);
///
/// let result = selector.select_as::<Vec<Person>>().unwrap();
/// assert_eq!(input_person()[1], result[0]);
///
/// let _ = selector.path("$..[?(@.age == 40)]");
///
/// let result = selector.select_as_value().unwrap();
/// assert_eq!(input_json()[0], result[0]);
///
/// let result = selector.select_as_str().unwrap();
/// assert_eq!(serde_json::to_string(&vec![&input_json()[0].clone()]).unwrap(), result);
///
/// let result = selector.select_as::<Vec<Person>>().unwrap();
/// assert_eq!(input_person()[0], result[0]);
///
/// selector.map(|v| {
///    let r = match v {
///        Value::Array(mut vec) => {
///            for mut v in &mut vec {
///                v.as_object_mut().unwrap().remove("age");
///            }
///            Value::Array(vec)
///        }
///        _ => Value::Null
///    };
///    Some(r)
/// });
/// assert_eq!(
///   serde_json::from_str::<Value>(r#"[{ "name": "이름1", "phone": "+33 12341234"}]"#).unwrap(),
///   selector.get().unwrap());
///
/// selector.value_from_str(input_str()).unwrap()
///     .map_as(|mut v: Vec<Person>| {
///        let mut p = v.pop().unwrap();
///        p.name = "name1".to_string();
///        p.age = None;
///        Some(vec![p])
///     });
/// assert_eq!(
///   vec![Person { name: "name1".to_string(), age: None, phone: "+33 12341234".to_string() }],
///   selector.get_as::<Vec<Person>>().unwrap());
/// ```
#[derive(Debug)]
pub struct Selector {
    pub(crate) node: Option<Node>,
    pub(crate) value: Option<RefValueWrapper>,
    path_builder: Arc<RefCell<PathMap>>,
}

impl Selector {
    pub fn new() -> Self {
        Selector { node: None, value: None, path_builder: Arc::new(RefCell::new(PathMap::new())) }
    }

    fn set_value(&mut self, value: RefValueWrapper) {
//        (*self.path_builder).borrow_mut().replace(&value);
        self.value = Some(value);
    }

    pub fn path(&mut self, path: &str) -> result::Result<&mut Self, String> {
        let mut parser = Parser::new(path);
        self.node = Some(parser.compile()?);
        Ok(self)
    }

    pub fn value(&mut self, value: &Value) -> result::Result<&mut Self, String> {
        self.set_value(value.into());
//        (*self.path_builder).borrow_mut().print();
        Ok(self)
    }

    pub fn value_from_ref_value(&mut self, value: RefValueWrapper) -> result::Result<&mut Self, String> {
        self.set_value(value);
        Ok(self)
    }

    pub fn value_from(&mut self, serializable: &impl serde::ser::Serialize) -> result::Result<&mut Self, String> {
        let ref_value: RefValue = serializable
            .serialize(ref_value::ser::RefValueSerializer)
            .map_err(|e| e.to_string())?;
        self.set_value(ref_value.into());
        Ok(self)
    }

    pub fn value_from_str(&mut self, json_str: &str) -> result::Result<&mut Self, String> {
        let value: RefValue = serde_json::from_str(json_str)
            .map_err(|e| e.to_string())?;
        self.set_value(value.into());
        Ok(self)
    }

    fn jf(&self) -> result::Result<JsonValueFilter, String> {
        match &self.value {
            Some(v) => Ok(JsonValueFilter::new(v.clone(),
                                               self.path_builder.clone())),
            _ => return Err(SelectorErrorMessage::EmptyValue.to_string())
        }
    }

    fn select(&self) -> result::Result<RefValueWrapper, String> {
        let mut jf = self.jf()?;

        match &self.node {
            Some(node) => {
                jf.visit(node.clone());
                Ok(jf.clone_value())
            }
            _ => Err(SelectorErrorMessage::EmptyPath.to_string())
        }
    }

    #[deprecated(since = "0.1.13", note = "Please use the select_as_str function instead")]
    pub fn select_to_str(&self) -> result::Result<String, String> {
        self.select_as_str()
    }

    #[deprecated(since = "0.1.13", note = "Please use the select_as_value function instead")]
    pub fn select_to_value(&self) -> result::Result<Value, String> {
        self.select_as_value()
    }

    #[deprecated(since = "0.1.13", note = "Please use the select_as function instead")]
    pub fn select_to<T: serde::de::DeserializeOwned>(&self) -> result::Result<T, String> {
        self.select_as()
    }

    pub fn select_as_str(&self) -> result::Result<String, String> {
        serde_json::to_string(self.select()?.deref()).map_err(|e| e.to_string())
    }

    pub fn select_as_value2(&self) {
        let _ = &self.select();
    }

    pub fn select_as_value(&self) -> result::Result<Value, String> {
        Ok((&self.select()?).into())
    }

    pub fn select_as<T: serde::de::DeserializeOwned>(&self) -> result::Result<T, String> {
        T::deserialize(self.select()?.deref()).map_err(|e| e.to_string())
    }

    pub fn map<F>(&mut self, func: F) -> result::Result<&mut Self, String>
        where F: FnOnce(Value) -> Option<Value>
    {
        match func((&self.select()?).into()).map(|ref v| v.into()) {
            Some(value) => {
                self.set_value(value)
            },
            _ => {}
        }
        Ok(self)
    }

    pub fn map_as<F, D, S>(&mut self, func: F) -> result::Result<&mut Self, String>
        where F: FnOnce(D) -> Option<S>,
              D: serde::de::DeserializeOwned,
              S: serde::ser::Serialize
    {
        let ret = func(D::deserialize(self.select()?.deref()).map_err(|e| e.to_string())?)
            .map(|ref ser| ser.serialize(ref_value::ser::RefValueSerializer));

        match ret {
            Some(ret) => match ret {
                Ok(v) => self.set_value(v.into()),
                Err(e) => return Err(e.to_string())
            }
            _ => {}
        };
        Ok(self)
    }

    pub fn get(&self) -> result::Result<Value, String> {
        match &self.value {
            Some(value) => Ok(value.into()),
            _ => Err(SelectorErrorMessage::EmptyValue.to_string())
        }
    }

    pub fn get_as<T: serde::de::DeserializeOwned>(&self) -> result::Result<T, String> {
        match &self.value {
            Some(value) => T::deserialize(value.deref()).map_err(|e| e.to_string()),
            _ => Err(SelectorErrorMessage::EmptyValue.to_string())
        }
    }
}

enum SelectorErrorMessage {
    EmptyValue,
    EmptyPath,
}

impl fmt::Display for SelectorErrorMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SelectorErrorMessage::EmptyValue => write!(f, "Empty value"),
            SelectorErrorMessage::EmptyPath => write!(f, "Empty path"),
        }
    }
}
