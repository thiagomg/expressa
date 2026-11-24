//! Canonical storage for named bindings and compiled function parameters.
//! Parameter names are shared by all calls; their values belong to each Env.
use std::collections::HashMap;
use std::rc::Rc;

use super::value::Value;

#[derive(Default)]
pub struct Bindings {
    named: HashMap<String, Value>,
    parameters: Option<Parameters>,
}
struct Parameters {
    names: Rc<HashMap<String, u32>>,
    values: Vec<Value>,
}
impl Bindings {
    pub(crate) fn bind_parameters(&mut self, names: Rc<HashMap<String, u32>>, values: Vec<Value>) {
        debug_assert!(self.is_empty());
        debug_assert_eq!(names.len(), values.len());
        self.parameters = Some(Parameters { names, values });
    }
    pub(crate) fn parameter(&self, index: u32) -> &Value {
        &self
            .parameters
            .as_ref()
            .expect("compiled parameters")
            .values[index as usize]
    }
    pub(crate) fn parameter_mut(&mut self, index: u32) -> &mut Value {
        &mut self
            .parameters
            .as_mut()
            .expect("compiled parameters")
            .values[index as usize]
    }
    pub fn get(&self, name: &str) -> Option<&Value> {
        if let Some(params) = &self.parameters
            && let Some(index) = params.names.get(name)
        {
            return Some(&params.values[*index as usize]);
        }
        self.named.get(name)
    }
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        if let Some(params) = &mut self.parameters
            && let Some(index) = params.names.get(name)
        {
            return Some(&mut params.values[*index as usize]);
        }
        self.named.get_mut(name)
    }
    pub fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
    pub fn insert(&mut self, name: String, value: Value) -> Option<Value> {
        if let Some(params) = &mut self.parameters
            && let Some(index) = params.names.get(&name)
        {
            return Some(std::mem::replace(
                &mut params.values[*index as usize],
                value,
            ));
        }
        self.named.insert(name, value)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.named
            .iter()
            .chain(self.parameters.iter().flat_map(|p| {
                p.names
                    .iter()
                    .map(|(name, index)| (name, &p.values[*index as usize]))
            }))
    }
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.named
            .values()
            .chain(self.parameters.iter().flat_map(|p| p.values.iter()))
    }
    pub fn len(&self) -> usize {
        self.named.len() + self.parameters.as_ref().map_or(0, |p| p.values.len())
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn clear(&mut self) {
        self.named.clear();
        self.parameters = None;
    }
}
