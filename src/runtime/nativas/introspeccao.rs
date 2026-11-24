//! Runtime discovery shared by the printed and structured APIs.
use std::cell::RefCell;
use std::rc::Rc;

use super::super::env::Env;
use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::value::{MapKey, Value};
use super::{builtin_modulo, lookup_alvo, lookup_builtin_doc};
use crate::lexer::Span;

struct FunctionInfo {
    name: String,
    args: Vec<String>,
    doc: String,
}

impl FunctionInfo {
    fn value(&self) -> Value {
        Value::mapa(vec![
            (
                MapKey::Texto("nome".into()),
                Value::Texto(self.name.clone()),
            ),
            (
                MapKey::Texto("args".into()),
                Value::lista(self.args.iter().cloned().map(Value::Texto).collect()),
            ),
            (MapKey::Texto("doc".into()), Value::Texto(self.doc.clone())),
        ])
    }
}

fn functions(bindings: Vec<(String, Value)>, module: Option<&str>) -> Vec<FunctionInfo> {
    let mut rows = Vec::new();
    for (name, value) in bindings {
        if name.starts_with('_') {
            continue;
        }
        let (args, doc) = match value {
            Value::Funcao(c) => (
                c.params.iter().map(|p| p.name.clone()).collect(),
                c.doc.clone().unwrap_or_default(),
            ),
            Value::Builtin(canon) => {
                let module = module.unwrap_or_else(|| builtin_modulo(canon));
                let doc = if module.is_empty() || module == "nucleo" {
                    lookup_builtin_doc(canon)
                } else {
                    lookup_alvo(&format!("{module}::{canon}")).or_else(|| lookup_builtin_doc(canon))
                };
                let args = doc.map(|d| d.args_text()).unwrap_or_default();
                let args = args
                    .split(',')
                    .map(str::trim)
                    .filter(|a| !a.is_empty())
                    .map(str::to_owned)
                    .collect();
                (args, doc.map(|d| d.doc.clone()).unwrap_or_default())
            }
            _ => continue,
        };
        rows.push(FunctionInfo { name, args, doc });
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

impl Vm<'_, '_> {
    fn discovery_data_mode(&self, mode: Option<&Value>, span: Span) -> Result<bool, EvalError> {
        match mode {
            None => Ok(false),
            Some(Value::Texto(s)) if s == "dados" => Ok(true),
            Some(_) => Err(self.err("modo de consulta inválido; use :dados", span)),
        }
    }

    pub(crate) fn bi_modulos(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.len() > 1 {
            return Err(self.err("modulos() espera 0 ou 1 argumento(s)", span));
        }
        let data = self.discovery_data_mode(args.first(), span)?;
        let mut modules: Vec<_> = self
            .native_modules
            .iter()
            .map(|(n, e)| (n.clone(), Rc::clone(e)))
            .collect();
        modules.push(("nucleo".into(), self.builtins_env()));
        modules.extend(self.imported_modules());
        modules.sort_by(|a, b| a.0.cmp(&b.0));
        if data {
            let rows = modules
                .into_iter()
                .map(|(name, env)| {
                    let funcs = functions(env.borrow().bindings_sorted(), Some(&name));
                    Value::mapa(vec![
                        (MapKey::Texto("nome".into()), Value::Texto(name)),
                        (
                            MapKey::Texto("funcoes".into()),
                            Value::lista(funcs.iter().map(FunctionInfo::value).collect()),
                        ),
                    ])
                })
                .collect();
            Ok(Value::lista(rows))
        } else {
            for (name, _) in modules {
                writeln!(self.out, "{name}").map_err(|e| self.io_err(e, span))?;
            }
            self.out.flush().map_err(|e| self.io_err(e, span))?;
            Ok(Value::Nada)
        }
    }

    pub(crate) fn bi_funcoes(
        &mut self,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        if args.len() > 2 {
            return Err(self.err("funcoes() espera 0, 1 ou 2 argumento(s)", span));
        }
        let data = self.discovery_data_mode(args.get(1), span)?;
        let rows = match args.first() {
            None | Some(Value::Nada) => functions(Env::visible_bindings(env), None),
            Some(Value::Texto(name)) if name.is_empty() => {
                functions(Env::visible_bindings(env), None)
            }
            Some(Value::Texto(name)) => {
                let module = if matches!(name.as_str(), "nucleo" | "núcleo") {
                    Some(self.builtins_env())
                } else {
                    self.native_modules
                        .get(name)
                        .cloned()
                        .or_else(|| {
                            self.imported_modules()
                                .into_iter()
                                .find(|(n, _)| n == name)
                                .map(|(_, e)| e)
                        })
                        .or_else(|| match Env::get(env, name) {
                            Some(Value::Modulo(e)) => Some(e),
                            _ => None,
                        })
                }
                .ok_or_else(|| {
                    self.err(
                        format!("módulo `{name}` não existe ou não foi importado"),
                        span,
                    )
                })?;
                let native_name = self.native_module_name(&module);
                functions(module.borrow().bindings_sorted(), native_name.as_deref())
            }
            Some(Value::Modulo(module)) => {
                let native_name = self.native_module_name(module);
                functions(module.borrow().bindings_sorted(), native_name.as_deref())
            }
            Some(v) => {
                return Err(self.err(
                    format!(
                        "funcoes espera nome de módulo ou módulo, encontrado {}",
                        v.type_name()
                    ),
                    span,
                ));
            }
        };
        if data {
            Ok(Value::lista(rows.iter().map(FunctionInfo::value).collect()))
        } else {
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    writeln!(self.out).map_err(|e| self.io_err(e, span))?;
                }
                writeln!(self.out, "{}({})", row.name, row.args.join(", "))
                    .map_err(|e| self.io_err(e, span))?;
                for line in row.doc.lines() {
                    writeln!(self.out, "    {line}").map_err(|e| self.io_err(e, span))?;
                }
            }
            self.out.flush().map_err(|e| self.io_err(e, span))?;
            Ok(Value::Nada)
        }
    }

    fn native_module_name(&self, module: &Rc<RefCell<Env>>) -> Option<String> {
        self.native_modules
            .iter()
            .find(|(_, e)| Rc::ptr_eq(e, module))
            .map(|(n, _)| n.clone())
    }
}
