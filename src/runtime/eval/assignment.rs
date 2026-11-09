//! Resolved assignment destinations. No source expression is evaluated during writeback.
use super::*;

pub(super) enum Destination {
    Name {
        env: Rc<RefCell<Env>>,
        name: String,
        span: Span,
    },
    Index {
        object: Value,
        index: Value,
        span: Span,
    },
    Matrix {
        object: Value,
        row: Value,
        col: Value,
        span: Span,
    },
    Text {
        owner: Box<Destination>,
        text: String,
        index: Value,
        span: Span,
    },
    ReadOnly {
        message: &'static str,
        span: Span,
    },
}

impl Vm<'_, '_> {
    pub(super) fn resolve_target(
        &mut self,
        target: &AssignTarget,
        env: &Rc<RefCell<Env>>,
        compound: bool,
    ) -> Result<Destination, EvalError> {
        match target {
            AssignTarget::Name { name, span } => Ok(Destination::Name {
                env: Rc::clone(env),
                name: name.clone(),
                span: *span,
            }),
            AssignTarget::Index {
                object,
                index,
                span,
            } => {
                // Keep ordinary assignment's historical index-before-receiver order;
                // compound assignment reads its receiver and index left to right.
                let (value, owner, index) = if compound {
                    let (value, owner) = self.resolve_receiver(object, env)?;
                    let index = self.eval_expr(index, env)?;
                    (value, owner, index)
                } else {
                    let index = self.eval_expr(index, env)?;
                    let (value, owner) = self.resolve_receiver(object, env)?;
                    (value, owner, index)
                };
                Ok(Self::index_destination(value, owner, index, *span))
            }
            AssignTarget::Index2 {
                object,
                row,
                col,
                span,
            } => {
                let object = self.eval_expr(object, env)?;
                let row = self.eval_expr(row, env)?;
                let col = self.eval_expr(col, env)?;
                Ok(Destination::Matrix {
                    object,
                    row,
                    col,
                    span: *span,
                })
            }
            AssignTarget::MapField {
                object,
                field,
                span,
            } => {
                let object = self.eval_expr(object, env)?;
                self.map_destination(object, field, *span)
            }
        }
    }

    fn index_destination(
        object: Value,
        owner: Destination,
        index: Value,
        span: Span,
    ) -> Destination {
        match object {
            Value::Texto(text) => Destination::Text {
                owner: Box::new(owner),
                text,
                index,
                span,
            },
            object => Destination::Index {
                object,
                index,
                span,
            },
        }
    }

    fn map_destination(
        &self,
        object: Value,
        field: &str,
        span: Span,
    ) -> Result<Destination, EvalError> {
        match object {
            Value::Mapa(_) => Ok(Destination::Index {
                object,
                index: Value::Texto(field.into()),
                span,
            }),
            Value::Par(_, _) => {
                Err(self.err("par não pode ser alterado; :chave e :valor só leem", span))
            }
            Value::Modulo(_) => {
                Err(self.err(format!("módulo usa '::' — tente o_modulo::{field}"), span))
            }
            other => Err(self.err(
                format!(
                    "acesso ':' espera um mapa, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }

    /// Evaluate a receiver and retain a writeback path only when its value needs one.
    /// Lists/maps/matrices mutate their storage; text values need their owner's slot.
    fn resolve_receiver(
        &mut self,
        expr: &Expr,
        env: &Rc<RefCell<Env>>,
    ) -> Result<(Value, Destination), EvalError> {
        stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || {
            self.resolve_receiver_inner(expr, env)
        })
    }

    fn resolve_receiver_inner(
        &mut self,
        expr: &Expr,
        env: &Rc<RefCell<Env>>,
    ) -> Result<(Value, Destination), EvalError> {
        let destination = match expr {
            Expr::Ident { name, span } => Destination::Name {
                env: Rc::clone(env),
                name: name.clone(),
                span: *span,
            },
            Expr::Index {
                object,
                index,
                span,
            } => {
                let (object, owner) = self.resolve_receiver(object, env)?;
                let index = self.eval_expr(index, env)?;
                Self::index_destination(object, owner, index, *span)
            }
            Expr::Index2 {
                object,
                row,
                col,
                span,
            } => {
                let object = self.eval_expr(object, env)?;
                let row = self.eval_expr(row, env)?;
                let col = self.eval_expr(col, env)?;
                Destination::Matrix {
                    object,
                    row,
                    col,
                    span: *span,
                }
            }
            Expr::MapField {
                object,
                field,
                span,
            } => {
                let object = self.eval_expr(object, env)?;
                // Pair fields are readable receivers, but are not writable owners.
                if let Value::Par(key, value) = &object {
                    let value = match field.as_str() {
                        "chave" => key.to_value(),
                        "valor" => value.as_ref().clone(),
                        _ => {
                            return Err(self
                                .err(format!("par só tem :chave e :valor, não `{field}`"), *span));
                        }
                    };
                    return Ok((
                        value,
                        Destination::ReadOnly {
                            message: "par não pode ser alterado; :chave e :valor só leem",
                            span: *span,
                        },
                    ));
                }
                self.map_destination(object, field, *span)?
            }
            _ => {
                let value = self.eval_expr(expr, env)?;
                let message = if matches!(expr, Expr::String { .. }) {
                    "não é possível alterar um texto literal"
                } else {
                    "não é possível atribuir neste alvo"
                };
                return Ok((
                    value,
                    Destination::ReadOnly {
                        message,
                        span: expr.span(),
                    },
                ));
            }
        };
        let value = self.read_destination(&destination)?;
        Ok((value, destination))
    }

    pub(super) fn read_destination(&self, destination: &Destination) -> Result<Value, EvalError> {
        match destination {
            Destination::Name { env, name, span } => Env::get(env, name)
                .ok_or_else(|| self.err(format!("variável `{name}` não definida"), *span)),
            Destination::Index {
                object,
                index,
                span,
            } => self.index_get(object, index, *span),
            Destination::Matrix {
                object,
                row,
                col,
                span,
            } => self.index_get2(object, row, col, *span),
            Destination::Text {
                text, index, span, ..
            } => self.index_get(&Value::Texto(text.clone()), index, *span),
            Destination::ReadOnly { message, span } => Err(self.err(*message, *span)),
        }
    }

    pub(super) fn write_destination(
        &mut self,
        destination: Destination,
        value: Value,
    ) -> Result<(), EvalError> {
        stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || {
            self.write_destination_inner(destination, value)
        })
    }

    fn write_destination_inner(
        &mut self,
        destination: Destination,
        value: Value,
    ) -> Result<(), EvalError> {
        match destination {
            Destination::Name { env, name, span } => self.assign_name(&env, &name, value, span),
            Destination::Index {
                object,
                index,
                span,
            } => self.assign_index(&object, &index, value, span),
            Destination::Matrix {
                object,
                row,
                col,
                span,
            } => self.assign_index2(&object, &row, &col, value, span),
            Destination::Text {
                owner,
                text,
                index,
                span,
            } => {
                // Read the retained slot, not the source expression: RHS updates
                // to other characters must survive this character's writeback.
                let current = if matches!(*owner, Destination::ReadOnly { .. }) {
                    text
                } else {
                    match self.read_destination(&owner)? {
                        Value::Texto(current) => current,
                        other => {
                            return Err(self.err(
                                format!("destino de texto passou a ser {}", other.type_name()),
                                span,
                            ));
                        }
                    }
                };
                let text = self.replace_texto_char(&current, &index, &value, span)?;
                self.write_destination(*owner, Value::Texto(text))
            }
            Destination::ReadOnly { message, span } => Err(self.err(message, span)),
        }
    }
}
