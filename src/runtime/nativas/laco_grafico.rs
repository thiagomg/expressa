//! Explicit graphical loop; the hosts continue collecting input while the VM sleeps.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use crate::lexer::Span;
use crate::runtime::env::Env;
use crate::runtime::error::EvalError;
use crate::runtime::eval::Vm;
use crate::runtime::value::{ItensMapa, MapKey, Value};

pub(crate) struct LacoGrafico {
    recurso: Weak<ItensMapa>,
    tela: Weak<ItensMapa>,
    executando: Cell<bool>,
    parar: Cell<bool>,
}

impl Vm<'_, '_> {
    pub(crate) fn bi_laco_grafico(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let tela = super::graf::expect_graf(&args[0], span, self)?;
        let laco = Value::mapa_recurso(vec![(MapKey::Texto("tela".into()), args[0].clone())]);
        let Value::Mapa(mapa) = &laco else {
            unreachable!()
        };
        self.lacos_graficos
            .retain(|_, l| l.recurso.strong_count() > 0);
        self.lacos_graficos.insert(
            Rc::as_ptr(mapa) as usize,
            Rc::new(LacoGrafico {
                recurso: Rc::downgrade(mapa),
                tela: Rc::downgrade(&tela),
                executando: Cell::new(false),
                parar: Cell::new(false),
            }),
        );
        Ok(laco)
    }

    fn controle_laco(&self, v: &Value, span: Span) -> Result<Rc<LacoGrafico>, EvalError> {
        if let Value::Mapa(mapa) = v {
            let id = Rc::as_ptr(mapa) as usize;
            if let Some(laco) = self.lacos_graficos.get(&id)
                && Weak::ptr_eq(&laco.recurso, &Rc::downgrade(mapa))
            {
                return Ok(Rc::clone(laco));
            }
        }
        Err(self.err(
            "esperado laço gráfico criado por tela_grafica::laço_grafico()",
            span,
        ))
    }

    pub(crate) fn bi_graf_pare(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        self.controle_laco(&args[0], span)?.parar.set(true);
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_execute(
        &mut self,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let laco = self.controle_laco(&args[0], span)?;
        let Value::Mapa(config) = &args[1] else {
            return Err(self.err("execute espera um mapa com :fps, :eventos e :desenho", span));
        };
        let config = config.borrow();
        for (key, _) in config.iter() {
            if !matches!(key, MapKey::Texto(k) if matches!(k.as_str(), "fps" | "eventos" | "desenho"))
            {
                return Err(self.err(format!("opção desconhecida no laço gráfico: {key}"), span));
            }
        }
        let campo = |nome: &str| config.get(&MapKey::Texto(nome.into())).cloned();
        let fps = campo("fps").ok_or_else(|| self.err("execute: falta :fps", span))?;
        let fps = self.expect_f64(&fps, span)?;
        let periodo = if fps.is_finite() && fps > 0.0 {
            Duration::try_from_secs_f64(1.0 / fps)
                .ok()
                .filter(|d| !d.is_zero() && Instant::now().checked_add(*d).is_some())
        } else {
            None
        }
        .ok_or_else(|| {
            self.err(
                "fps deve ser positivo, finito e ter um período representável",
                span,
            )
        })?;
        let eventos = campo("eventos").ok_or_else(|| self.err("execute: falta :eventos", span))?;
        let desenho = campo("desenho").ok_or_else(|| self.err("execute: falta :desenho", span))?;
        self.valide_callback(&eventos, "eventos", 2, span)?;
        self.valide_callback(&desenho, "desenho", 1, span)?;
        drop(config);
        if self
            .lacos_graficos
            .values()
            .any(|outro| outro.executando.get() && Weak::ptr_eq(&outro.tela, &laco.tela))
        {
            return Err(self.err("já existe um laço gráfico em execução nesta tela", span));
        }
        let tela =
            Value::Mapa(laco.tela.upgrade().ok_or_else(|| {
                self.err("a tela do laço gráfico não está mais disponível", span)
            })?);
        laco.parar.set(false);
        laco.executando.set(true);
        let resultado = (|| {
            self.check_espera(span)?;
            self.bi_graf_mostre(std::slice::from_ref(&tela), span)?;
            let mut anterior: Option<Instant> = None;
            loop {
                self.check_espera(span)?;
                if laco.parar.get() {
                    break;
                }
                let inicio = Instant::now();
                let ev = self.bi_graf_evento(std::slice::from_ref(&tela), span)?;
                let agora = Instant::now();
                let dt = anterior.map_or(0.0, |t| agora.duration_since(t).as_secs_f64());
                anterior = Some(agora);
                let Value::Mapa(mapa) = &ev else {
                    unreachable!()
                };
                let sair = matches!(
                    mapa.borrow().get(&MapKey::Texto("sair".into())),
                    Some(Value::Bool(true))
                );
                let dt = crate::runtime::numero::Numero::try_from(dt)
                    .map_err(|e| self.err(e.to_string(), span))?;
                self.call_value(&eventos, &[ev, Value::Numero(dt)], span, "eventos", env)?;
                self.check_espera(span)?;
                if sair || laco.parar.get() {
                    break;
                }
                self.bi_graf_limpe(std::slice::from_ref(&tela), span)?;
                self.call_value(&desenho, std::slice::from_ref(&tela), span, "desenho", env)?;
                self.check_espera(span)?;
                if laco.parar.get() {
                    break;
                }
                self.bi_graf_mostre(std::slice::from_ref(&tela), span)?;
                let restante = periodo.saturating_sub(inicio.elapsed());
                self.sleep_secs(restante.as_secs_f64(), span)?;
            }
            Ok(args[0].clone())
        })();
        // Also release the guard when a callback, deadline or stop hook fails.
        laco.executando.set(false);
        resultado
    }

    fn valide_callback(
        &self,
        f: &Value,
        nome: &str,
        aridade: usize,
        span: Span,
    ) -> Result<(), EvalError> {
        match f {
            Value::Funcao(c) if c.params.len() != aridade => Err(self.err(
                format!(
                    ":{nome} deve receber {aridade} argumento(s), recebeu função com {}",
                    c.params.len()
                ),
                span,
            )),
            Value::Funcao(_) | Value::Builtin(_) => Ok(()),
            _ => Err(self.err(format!(":{nome} deve ser uma função"), span)),
        }
    }
}
