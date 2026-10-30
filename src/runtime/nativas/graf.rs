use std::rc::Rc;
use std::time::{Duration, Instant};

use super::super::fonte8x8;
use crate::lexer::Span;
use crate::runtime::error::EvalError;
use crate::runtime::eval::Vm;
use crate::runtime::graf::{ALTURA_MAX, LARGURA_MAX, OpGraf, TelaGrafica, cor_graf_conhecida};
use crate::runtime::graf_host::{EventoGraf, GrafHost};
use crate::runtime::imagem::{self, Imagem};
use crate::runtime::value::{ListaRc, MapKey, MapaRc, Value};

impl Vm<'_, '_> {
    pub(crate) fn bi_graf_nova(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let largura = self.expect_graf_dim(&args[0], "largura", span)?;
        let altura = self.expect_graf_dim(&args[1], "altura", span)?;
        Ok(Value::mapa(vec![
            (
                MapKey::Texto("largura".into()),
                Value::Numero(largura.into()),
            ),
            (MapKey::Texto("altura".into()), Value::Numero(altura.into())),
            (MapKey::Texto("cor".into()), Value::Texto("preto".into())),
            (MapKey::Texto("fundo".into()), Value::Texto("branco".into())),
            (MapKey::Texto("ops".into()), Value::lista(vec![])),
        ]))
    }

    pub(crate) fn bi_graf_cor(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let mapa = expect_graf(&args[0], span, self)?;
        let cor = self.expect_texto(&args[1], span)?;
        check_cor(&cor, span, self)?;
        set_texto(&mut mapa.borrow_mut(), "cor", cor);
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_fundo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let mapa = expect_graf(&args[0], span, self)?;
        let cor = self.expect_texto(&args[1], span)?;
        check_cor(&cor, span, self)?;
        set_texto(&mut mapa.borrow_mut(), "fundo", cor);
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_linha(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 5, span)?;
        let mapa = expect_graf(&args[0], span, self)?;
        let x1 = self.expect_px(&args[1], span)?;
        let y1 = self.expect_px(&args[2], span)?;
        let x2 = self.expect_px(&args[3], span)?;
        let y2 = self.expect_px(&args[4], span)?;
        let cor = pen(&mapa);
        push_op(
            &mapa,
            Value::mapa(vec![
                (MapKey::Texto("tipo".into()), Value::Texto("linha".into())),
                (
                    MapKey::Texto("x1".into()),
                    Value::Numero((x1 as f64).into()),
                ),
                (
                    MapKey::Texto("y1".into()),
                    Value::Numero((y1 as f64).into()),
                ),
                (
                    MapKey::Texto("x2".into()),
                    Value::Numero((x2 as f64).into()),
                ),
                (
                    MapKey::Texto("y2".into()),
                    Value::Numero((y2 as f64).into()),
                ),
                (MapKey::Texto("cor".into()), Value::Texto(cor)),
            ]),
        );
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_circulo(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 4, span)?;
        let mapa = expect_graf(&args[0], span, self)?;
        let x = self.expect_px(&args[1], span)?;
        let y = self.expect_px(&args[2], span)?;
        let r = self.expect_px(&args[3], span)?;
        if r < 0 {
            return Err(self.err("raio do círculo deve ser ≥ 0", span));
        }
        let cor = pen(&mapa);
        push_op(
            &mapa,
            Value::mapa(vec![
                (MapKey::Texto("tipo".into()), Value::Texto("circulo".into())),
                (MapKey::Texto("x".into()), Value::Numero((x as f64).into())),
                (MapKey::Texto("y".into()), Value::Numero((y as f64).into())),
                (MapKey::Texto("r".into()), Value::Numero((r as f64).into())),
                (MapKey::Texto("cor".into()), Value::Texto(cor)),
            ]),
        );
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_ponto(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let mapa = expect_graf(&args[0], span, self)?;
        let x = self.expect_px(&args[1], span)?;
        let y = self.expect_px(&args[2], span)?;
        let cor = pen(&mapa);
        push_op(
            &mapa,
            Value::mapa(vec![
                (MapKey::Texto("tipo".into()), Value::Texto("ponto".into())),
                (MapKey::Texto("x".into()), Value::Numero((x as f64).into())),
                (MapKey::Texto("y".into()), Value::Numero((y as f64).into())),
                (MapKey::Texto("cor".into()), Value::Texto(cor)),
            ]),
        );
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_retangulo(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 5, span)?;
        let mapa = expect_graf(&args[0], span, self)?;
        let x1 = self.expect_px(&args[1], span)?;
        let y1 = self.expect_px(&args[2], span)?;
        let x2 = self.expect_px(&args[3], span)?;
        let y2 = self.expect_px(&args[4], span)?;
        let cor = pen(&mapa);
        push_op(
            &mapa,
            Value::mapa(vec![
                (
                    MapKey::Texto("tipo".into()),
                    Value::Texto("retangulo".into()),
                ),
                (
                    MapKey::Texto("x1".into()),
                    Value::Numero((x1 as f64).into()),
                ),
                (
                    MapKey::Texto("y1".into()),
                    Value::Numero((y1 as f64).into()),
                ),
                (
                    MapKey::Texto("x2".into()),
                    Value::Numero((x2 as f64).into()),
                ),
                (
                    MapKey::Texto("y2".into()),
                    Value::Numero((y2 as f64).into()),
                ),
                (MapKey::Texto("cor".into()), Value::Texto(cor)),
            ]),
        );
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_texto(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.len() != 4 && args.len() != 5 {
            return Err(self.err(
                format!("esperado 4 ou 5 argumento(s), recebeu {}", args.len()),
                span,
            ));
        }
        let mapa = expect_graf(&args[0], span, self)?;
        let x = self.expect_px(&args[1], span)?;
        let y = self.expect_px(&args[2], span)?;
        let texto = self.expect_texto(&args[3], span)?;
        let tamanho = if args.len() == 5 {
            let n = self.expect_int(&args[4], span)?;
            if n < 1 || n > fonte8x8::TAMANHO_MAX as i64 {
                return Err(self.err(
                    format!(
                        "tamanho do texto deve ser um inteiro de 1 a {}",
                        fonte8x8::TAMANHO_MAX
                    ),
                    span,
                ));
            }
            n as i32
        } else {
            fonte8x8::TAMANHO_PADRAO
        };
        let cor = pen(&mapa);
        push_op(
            &mapa,
            Value::mapa(vec![
                (MapKey::Texto("tipo".into()), Value::Texto("texto".into())),
                (MapKey::Texto("x".into()), Value::Numero((x as f64).into())),
                (MapKey::Texto("y".into()), Value::Numero((y as f64).into())),
                (
                    MapKey::Texto("tamanho".into()),
                    Value::Numero((tamanho as f64).into()),
                ),
                (MapKey::Texto("cor".into()), Value::Texto(cor)),
                (MapKey::Texto("texto".into()), Value::Texto(texto)),
            ]),
        );
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_imagem(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        if args.len() != 4 && args.len() != 5 {
            return Err(self.err(
                format!("esperado 4 ou 5 argumento(s), recebeu {}", args.len()),
                span,
            ));
        }
        let mapa = expect_graf(&args[0], span, self)?;
        let figura = figura_de(&args[1], span, self)?;
        let x = self.expect_px(&args[2], span)?;
        let y = self.expect_px(&args[3], span)?;
        let angulo = if args.len() == 5 {
            self.expect_f64(&args[4], span)?
        } else {
            0.0
        };
        if !angulo.is_finite() {
            return Err(self.err("ângulo da imagem deve ser um número finito", span));
        }
        push_op(
            &mapa,
            Value::mapa(vec![
                (MapKey::Texto("tipo".into()), Value::Texto("imagem".into())),
                (MapKey::Texto("figura".into()), Value::Imagem(figura)),
                (MapKey::Texto("x".into()), Value::Numero((x as f64).into())),
                (MapKey::Texto("y".into()), Value::Numero((y as f64).into())),
                (MapKey::Texto("angulo".into()), Value::Numero(angulo.into())),
            ]),
        );
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_limpe(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let mapa = expect_graf(&args[0], span, self)?;
        if let Some(ops) = get_lista(&mapa, "ops") {
            ops.borrow_mut().clear();
        }
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_pixel(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let tela = tela_de(&args[0], span, self)?;
        let x = self.expect_px(&args[1], span)?;
        let y = self.expect_px(&args[2], span)?;
        match tela.raster().pixel(x, y) {
            Some(nome) => Ok(Value::Texto(nome.into())),
            None => Err(self.err(
                format!(
                    "pixel ({x}, {y}) fora da tela (0..{}, 0..{})",
                    tela.largura.saturating_sub(1),
                    tela.altura.saturating_sub(1)
                ),
                span,
            )),
        }
    }

    pub(crate) fn bi_graf_salve(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let tela = tela_de(&args[0], span, self)?;
        let nome = self.expect_texto(&args[1], span)?;
        if !nome.to_ascii_lowercase().ends_with(".png") {
            return Err(self.err("salve espera um arquivo .png", span));
        }
        let path = self.resolve_data_path(&nome, span)?;
        self.ensure_parent_dir(&path, span)?;
        tela.write_png(&path).map_err(|e| {
            self.err(
                format!("não foi possível salvar '{}': {e}", path.display()),
                span,
            )
        })?;
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_como_imagem(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let tela = tela_de(&args[0], span, self)?;
        Ok(Value::Imagem(tela.como_imagem()))
    }

    pub(crate) fn bi_graf_mostre(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let tela = tela_de(&args[0], span, self)?;
        if let Some(host) = self.graf_host.as_mut() {
            host.mostre(tela).map_err(|e| self.err(e, span))?;
        } else if self.abrir_cartesiano {
            self.cli_graf.mostre(tela).map_err(|e| self.err(e, span))?;
        }
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_evento(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        if args.is_empty() || args.len() > 2 {
            return Err(self.err(
                format!("esperado 1 ou 2 argumento(s), recebeu {}", args.len()),
                span,
            ));
        }
        // Window id only: do not rebuild `:ops` (thousands of ponto maps
        // on a Conway-style frame) just to wait for a click.
        let tela = tela_para_evento(&args[0], span, self)?;
        let timeout_secs = if args.len() == 2 {
            let s = self.expect_f64(&args[1], span)?;
            if !s.is_finite() || s < 0.0 {
                return Err(self.err("evento() espera um tempo ≥ 0 (segundos)", span));
            }
            Some(s)
        } else {
            None
        };
        if timeout_secs == Some(0.0) {
            return self
                .pedir_evento(&tela, Some(Duration::ZERO), span)
                .map(evento_valor);
        }
        let deadline = timeout_secs.map(|s| {
            Instant::now() + Duration::from_secs_f64(s.min((i32::MAX as u64 / 1000) as f64))
        });
        loop {
            self.check_deadline(span)?;
            if self.interrupted() {
                return Err(EvalError::Quit(0));
            }
            let slice = match deadline {
                None => Duration::from_millis(50),
                Some(end) => {
                    let left = end.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Ok(evento_valor(EventoGraf::nada()));
                    }
                    left.min(Duration::from_millis(50))
                }
            };
            let ev = self.pedir_evento(&tela, Some(slice), span)?;
            if ev.tipo != "nada" {
                return Ok(evento_valor(ev));
            }
            if deadline.is_some_and(|end| Instant::now() >= end) {
                return Ok(evento_valor(EventoGraf::nada()));
            }
        }
    }

    fn pedir_evento(
        &mut self,
        tela: &TelaGrafica,
        timeout: Option<Duration>,
        span: Span,
    ) -> Result<EventoGraf, EvalError> {
        if let Some(host) = self.graf_host.as_mut() {
            host.evento(tela, timeout).map_err(|e| self.err(e, span))
        } else if self.abrir_cartesiano {
            self.cli_graf
                .evento(tela, timeout)
                .map_err(|e| self.err(e, span))
        } else {
            Err(self.err("evento() precisa de uma janela gráfica", span))
        }
    }

    fn expect_graf_dim(&self, v: &Value, nome: &str, span: Span) -> Result<u32, EvalError> {
        let n = self.expect_f64(v, span)?;
        if !n.is_finite() || n < 1.0 || n.fract() != 0.0 {
            return Err(self.err(format!("{nome} deve ser um inteiro ≥ 1"), span));
        }
        let n = n as u32;
        let max = if nome == "largura" {
            LARGURA_MAX
        } else {
            ALTURA_MAX
        };
        if n > max {
            return Err(self.err(format!("{nome} máxima é {max}"), span));
        }
        Ok(n)
    }

    fn expect_px(&self, v: &Value, span: Span) -> Result<i32, EvalError> {
        let n = self.expect_f64(v, span)?;
        if !n.is_finite() {
            return Err(self.err("coordenada deve ser um número finito", span));
        }
        Ok(n.round() as i32)
    }
}

pub(crate) fn is_graf(v: &Value) -> bool {
    let Value::Mapa(xs) = v else {
        return false;
    };
    tem_chave(xs, "largura") && tem_chave(xs, "altura") && tem_chave(xs, "ops")
}

fn expect_graf(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<MapaRc, EvalError> {
    match v {
        Value::Mapa(xs) if is_graf(v) => Ok(Rc::clone(xs)),
        Value::Mapa(_) => Err(vm.err("não é uma tela gráfica (tela_grafica::nova())", span)),
        other => Err(vm.err(
            format!("esperada tela gráfica, encontrado {}", other.type_name()),
            span,
        )),
    }
}

fn tela_para_evento(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<TelaGrafica, EvalError> {
    let mapa = expect_graf(v, span, vm)?;
    let Value::Mapa(xs) = v else {
        unreachable!();
    };
    let largura =
        get_u32(&mapa, "largura").ok_or_else(|| vm.err("tela gráfica sem :largura", span))?;
    let altura =
        get_u32(&mapa, "altura").ok_or_else(|| vm.err("tela gráfica sem :altura", span))?;
    Ok(TelaGrafica {
        id: Rc::as_ptr(xs) as u64,
        largura,
        altura,
        fundo: String::new(),
        ops: Vec::new(),
    })
}

fn tela_de(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<TelaGrafica, EvalError> {
    let mapa = expect_graf(v, span, vm)?;
    let Value::Mapa(xs) = v else {
        unreachable!();
    };
    let largura =
        get_u32(&mapa, "largura").ok_or_else(|| vm.err("tela gráfica sem :largura", span))?;
    let altura =
        get_u32(&mapa, "altura").ok_or_else(|| vm.err("tela gráfica sem :altura", span))?;
    let fundo = get_texto(&mapa, "fundo").unwrap_or_else(|| "branco".into());
    let ops_v = get_lista(&mapa, "ops").ok_or_else(|| vm.err("tela gráfica sem :ops", span))?;
    let mut ops = Vec::new();
    for item in ops_v.borrow().iter() {
        ops.push(op_de(item, span, vm)?);
    }
    Ok(TelaGrafica {
        id: Rc::as_ptr(xs) as u64,
        largura,
        altura,
        fundo,
        ops,
    })
}

fn op_de(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<OpGraf, EvalError> {
    let Value::Mapa(xs) = v else {
        return Err(vm.err(
            format!("comando de tela gráfica inválido ({})", v.type_name()),
            span,
        ));
    };
    let tipo = get_texto(xs, "tipo").unwrap_or_default();
    let cor = get_texto(xs, "cor").unwrap_or_else(|| "preto".into());
    match tipo.as_str() {
        "ponto" => Ok(OpGraf::Ponto {
            x: get_i32(xs, "x").unwrap_or(0),
            y: get_i32(xs, "y").unwrap_or(0),
            cor,
        }),
        "linha" => Ok(OpGraf::Linha {
            x1: get_i32(xs, "x1").unwrap_or(0),
            y1: get_i32(xs, "y1").unwrap_or(0),
            x2: get_i32(xs, "x2").unwrap_or(0),
            y2: get_i32(xs, "y2").unwrap_or(0),
            cor,
        }),
        "circulo" => Ok(OpGraf::Circulo {
            x: get_i32(xs, "x").unwrap_or(0),
            y: get_i32(xs, "y").unwrap_or(0),
            r: get_i32(xs, "r").unwrap_or(0),
            cor,
        }),
        "imagem" => {
            let figura = get_imagem(xs, "figura")
                .ok_or_else(|| vm.err("comando de imagem sem :figura", span))?;
            Ok(OpGraf::Imagem {
                x: get_i32(xs, "x").unwrap_or(0),
                y: get_i32(xs, "y").unwrap_or(0),
                angulo: get_f64(xs, "angulo").unwrap_or(0.0),
                figura,
            })
        }
        "texto" => Ok(OpGraf::Texto {
            x: get_i32(xs, "x").unwrap_or(0),
            y: get_i32(xs, "y").unwrap_or(0),
            tamanho: get_i32(xs, "tamanho")
                .unwrap_or(fonte8x8::TAMANHO_PADRAO)
                .clamp(1, fonte8x8::TAMANHO_MAX),
            cor,
            texto: get_texto(xs, "texto").unwrap_or_default(),
        }),
        "retangulo" | "retângulo" => Ok(OpGraf::Retangulo {
            x1: get_i32(xs, "x1").unwrap_or(0),
            y1: get_i32(xs, "y1").unwrap_or(0),
            x2: get_i32(xs, "x2").unwrap_or(0),
            y2: get_i32(xs, "y2").unwrap_or(0),
            cor,
        }),
        other => Err(vm.err(
            format!("comando de tela gráfica desconhecido ({other})"),
            span,
        )),
    }
}

fn check_cor(cor: &str, span: Span, vm: &Vm<'_, '_>) -> Result<(), EvalError> {
    if cor_graf_conhecida(cor) {
        Ok(())
    } else {
        Err(vm.err(
            format!(
                "cor desconhecida (\"{cor}\"); use normal, preto, vermelho, verde, amarelo, azul, magenta, ciano, branco ou transparente"
            ),
            span,
        ))
    }
}

fn pen(mapa: &MapaRc) -> String {
    get_texto(mapa, "cor").unwrap_or_else(|| "preto".into())
}

fn push_op(mapa: &MapaRc, op: Value) {
    if let Some(ops) = get_lista(mapa, "ops") {
        ops.borrow_mut().push(op);
    }
}

fn tem_chave(mapa: &MapaRc, chave: &str) -> bool {
    let k = MapKey::Texto(chave.into());
    mapa.borrow().iter().any(|(kk, _)| *kk == k)
}

fn get_lista(mapa: &MapaRc, chave: &str) -> Option<ListaRc> {
    let k = MapKey::Texto(chave.into());
    match mapa
        .borrow()
        .iter()
        .find(|(kk, _)| *kk == k)
        .map(|(_, v)| v)
    {
        Some(Value::Lista(xs)) => Some(Rc::clone(xs)),
        _ => None,
    }
}

fn figura_de(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<Imagem, EvalError> {
    match v {
        Value::Imagem(img) => Ok(img.clone()),
        Value::Mapa(_) if is_graf(v) => Ok(tela_de(v, span, vm)?.como_imagem()),
        Value::Texto(caminho) => {
            let path = vm.resolve_data_path(caminho, span)?;
            imagem::leia_png(&path).map_err(|e| {
                vm.err(
                    format!("não foi possível ler '{}': {e}", path.display()),
                    span,
                )
            })
        }
        other => Err(vm.err(
            format!(
                "imagem espera um caminho, uma imagem ou uma tela gráfica, encontrado {}",
                other.type_name()
            ),
            span,
        )),
    }
}

fn get_imagem(mapa: &MapaRc, chave: &str) -> Option<Imagem> {
    let k = MapKey::Texto(chave.into());
    match mapa
        .borrow()
        .iter()
        .find(|(kk, _)| *kk == k)
        .map(|(_, v)| v)
    {
        Some(Value::Imagem(img)) => Some(img.clone()),
        _ => None,
    }
}

fn get_texto(mapa: &MapaRc, chave: &str) -> Option<String> {
    let k = MapKey::Texto(chave.into());
    match mapa
        .borrow()
        .iter()
        .find(|(kk, _)| *kk == k)
        .map(|(_, v)| v)
    {
        Some(Value::Texto(s)) => Some(s.clone()),
        _ => None,
    }
}

fn get_u32(mapa: &MapaRc, chave: &str) -> Option<u32> {
    let k = MapKey::Texto(chave.into());
    match mapa
        .borrow()
        .iter()
        .find(|(kk, _)| *kk == k)
        .map(|(_, v)| v)
    {
        Some(Value::Numero(n)) => {
            let f = n.to_f64();
            if f.is_finite() && f >= 1.0 {
                Some(f as u32)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn get_f64(mapa: &MapaRc, chave: &str) -> Option<f64> {
    let k = MapKey::Texto(chave.into());
    match mapa
        .borrow()
        .iter()
        .find(|(kk, _)| *kk == k)
        .map(|(_, v)| v)
    {
        Some(Value::Numero(n)) => {
            let f = n.to_f64();
            f.is_finite().then_some(f)
        }
        _ => None,
    }
}

fn get_i32(mapa: &MapaRc, chave: &str) -> Option<i32> {
    let k = MapKey::Texto(chave.into());
    match mapa
        .borrow()
        .iter()
        .find(|(kk, _)| *kk == k)
        .map(|(_, v)| v)
    {
        Some(Value::Numero(n)) => {
            let f = n.to_f64();
            if f.is_finite() {
                Some(f.round() as i32)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn evento_valor(e: EventoGraf) -> Value {
    Value::mapa(vec![
        (MapKey::Texto("tipo".into()), Value::Texto(e.tipo)),
        (
            MapKey::Texto("x".into()),
            Value::Numero((e.x as f64).into()),
        ),
        (
            MapKey::Texto("y".into()),
            Value::Numero((e.y as f64).into()),
        ),
        (MapKey::Texto("botao".into()), Value::Texto(e.botao)),
        (MapKey::Texto("tecla".into()), Value::Texto(e.tecla)),
    ])
}

fn set_texto(entries: &mut Vec<(MapKey, Value)>, chave: &str, s: String) {
    let key = MapKey::Texto(chave.into());
    if let Some((_, v)) = entries.iter_mut().find(|(k, _)| *k == key) {
        *v = Value::Texto(s);
    } else {
        entries.push((key, Value::Texto(s)));
    }
}

#[cfg(test)]
#[allow(clippy::identity_op)] // `1 * 4`: pixel 1, 4 bytes each
mod tests {
    use crate::runtime::NoopHook;
    use crate::runtime::eval::{run_to_string, run_with_io};
    use crate::runtime::graf_host::{CapturaGraf, EventoGraf};

    fn run(src: &str) -> String {
        let src = format!("importe \"tela_grafica\"\n{src}");
        run_to_string(&src, "teste.lep").unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    fn run_err(src: &str) -> String {
        let src = format!("importe \"tela_grafica\"\n{src}");
        match run_to_string(&src, "teste.lep") {
            Err(e) => e.message,
            Ok(out) => panic!("expected error, got {out:?}\n{src}"),
        }
    }

    #[test]
    fn nova_e_mapa_e_pixel_do_fundo() {
        assert_eq!(
            run(r#"
g = nova(16, 16)
escreva(tipo(g))
escreva(g.pixel(0, 0))
"#),
            "mapa\nbranco\n"
        );
    }

    #[test]
    fn linha_e_pixel_e_limpe() {
        let out = run(r#"
g = nova(16, 16)
g.cor(:azul)
g.linha(0, 0, 15, 15)
escreva(g.pixel(0, 0))
escreva(g.pixel(15, 0))
g.apague()
escreva(g.pixel(0, 0))
"#);
        assert_eq!(out, "azul\nbranco\nbranco\n");
    }

    #[test]
    fn retangulo_preenche_inclusivo_e_recorta() {
        let out = run(r#"
g = nova(8, 8)
g.cor(:verde)
g.retangulo(2, 2, 4, 3)
escreva(g.pixel(2, 2))
escreva(g.pixel(4, 3))
escreva(g.pixel(5, 2))
g.apague()
g.cor(:azul)
g.retângulo(6, 6, 20, 20)
escreva(g.pixel(6, 6))
escreva(g.pixel(7, 7))
escreva(g.pixel(0, 0))
"#);
        assert_eq!(out, "verde\nverde\nbranco\nazul\nazul\nbranco\n");
    }

    #[test]
    fn fundo_transparente_e_como_imagem() {
        let out = run(r#"
g = nova(8, 8)
g.fundo(:transparente)
g.cor(:verde)
g.ponto(3, 3)
escreva(g.pixel(0, 0))
escreva(g.pixel(3, 3))
img = g.como_imagem()
escreva(tipo(img))
g.cor(:transparente)
g.ponto(3, 3)
escreva(g.pixel(3, 3))
g.fundo(:azul)
g.apague()
escreva(g.pixel(0, 0))
"#);
        assert_eq!(
            out,
            "transparente\nverde\nimagem\ntransparente\nazul\n"
        );
        assert!(run_err("g = nova(8, 8)\ng.fundo(:roxo)\n").contains("cor desconhecida"));
    }

    #[test]
    fn imagem_aceita_outra_tela() {
        let src = r#"
importe "tela_grafica"
celula = nova(4, 4)
celula.fundo(:transparente)
celula.cor(:verde)
celula.retangulo(1, 1, 2, 2)
w = nova(8, 8)
w.cor(:vermelho)
w.retangulo(0, 0, 7, 7)
w.imagem(celula, 4, 4)
w.mostre()
"#;
        let mut cap = CapturaGraf::default();
        let mut input = std::io::Cursor::new(String::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_with_io(
            src,
            "t.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            None,
            false,
            Some(&mut cap),
            &[],
        )
        .unwrap();
        let rgb = cap.telas[0].rgb32();
        assert_eq!(rgb[0], 0x00CC3333, "canto continua vermelho");
        assert_eq!(rgb[(3 * 8 + 3) as usize], 0x0022AA44, "célula verde no centro");
    }

    #[test]
    fn pixel_fora_e_erro() {
        assert!(run_err("g = nova(8, 8)\nescreva(g.pixel(8, 0))\n").contains("fora"));
        assert!(run_err("g = nova(0, 8)\n").contains("inteiro"));
        assert!(run_err("linha(1, 0, 0, 1, 1)\n").contains("tela gráfica"));
    }

    #[test]
    fn mostre_mesma_instancia_mesmo_id() {
        let src = r#"
importe "tela_grafica"
g = nova(8, 8)
g.ponto(1, 1)
g.mostre()
g.linha(0, 0, 7, 7)
g.mostre()
"#;
        let mut cap = CapturaGraf::default();
        let mut input = std::io::Cursor::new(String::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_with_io(
            src,
            "t.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            None,
            false,
            Some(&mut cap),
            &[],
        )
        .unwrap();
        assert_eq!(cap.telas.len(), 2);
        assert_eq!(cap.telas[0].id, cap.telas[1].id);
        assert_eq!(cap.telas[0].ops.len(), 1);
        assert_eq!(cap.telas[1].ops.len(), 2);
    }

    #[test]
    fn duas_novas_ids_diferentes() {
        let src = r#"
importe "tela_grafica"
a = nova(4, 4)
b = nova(4, 4)
a.mostre()
b.mostre()
"#;
        let mut cap = CapturaGraf::default();
        let mut input = std::io::Cursor::new(String::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_with_io(
            src,
            "t.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            None,
            false,
            Some(&mut cap),
            &[],
        )
        .unwrap();
        assert_eq!(cap.telas.len(), 2);
        assert_ne!(cap.telas[0].id, cap.telas[1].id);
    }

    #[test]
    fn salve_png() {
        let dir = std::env::temp_dir().join(format!("expressa-graf-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("casa.png");
        let src = format!(
            "importe \"tela_grafica\"\ng = nova(8, 8)\ng.cor(:vermelho)\ng.linha(0, 0, 7, 7)\ng.salve(\"{}\")\n",
            path.display()
        );
        run_to_string(&src, dir.join("t.lep").to_str().unwrap()).unwrap();
        let bytes = std::fs::read(&path).expect("png");
        assert!(bytes.starts_with(&[0x89, b'P', b'N', b'G']));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn texto_e_pixel_e_tamanho() {
        let out = run(r#"
g = nova(64, 16)
g.cor(:azul)
g.texto(0, 0, "A")
escreva(g.pixel(2, 0))
escreva(g.pixel(0, 0))
g.apague()
g.texto(0, 0, "oi", 16)
escreva(g.pixel(0, 0))
"#);
        assert_eq!(out, "azul\nbranco\nbranco\n");
        assert!(run_err("g = nova(8, 8)\ng.texto(0, 0, \"a\", 0)\n").contains("tamanho"));
        assert!(run_err("g = nova(8, 8)\ng.texto(0, 0, 1)\n").contains("texto"));
    }

    #[test]
    fn imagem_guarda_pixels_e_nao_rele_o_arquivo() {
        let dir = std::env::temp_dir().join(format!("expressa-graf-img-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut raw = vec![0u8; 3 * 3 * 4];
        raw[1 * 4] = 255;
        raw[1 * 4 + 3] = 255;
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 3, 3);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&raw).unwrap();
        }
        std::fs::write(dir.join("sprite.png"), &bytes).unwrap();
        let src = r#"
importe "tela_grafica"
g = nova(16, 16)
g.imagem("sprite.png", 8, 8, 0)
g.mostre()
"#;
        let mut cap = CapturaGraf::default();
        let mut input = std::io::Cursor::new(String::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_with_io(
            src,
            dir.join("t.lep").to_str().unwrap(),
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            None,
            false,
            Some(&mut cap),
            &[],
        )
        .unwrap();
        std::fs::remove_file(dir.join("sprite.png")).unwrap();
        let rgb = cap.telas[0].rgb32();
        assert_eq!(rgb[(7 * 16 + 8) as usize], 0x00FF0000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn evento_sem_janela_e_erro() {
        assert!(run_err("g = nova(8, 8)\nescreva(g.evento(0))\n").contains("janela"));
    }

    #[test]
    fn evento_devolve_clique_da_fila() {
        let src = r#"
importe "tela_grafica"
g = nova(8, 8)
g.mostre()
ev = g.evento(0)
escreva(ev:tipo)
escreva(ev:x)
escreva(ev:y)
escreva(ev:botao)
"#;
        let mut cap = CapturaGraf::default();
        cap.eventos.push_back(EventoGraf::clique(3, 4, "esquerdo"));
        let mut input = std::io::Cursor::new(String::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_with_io(
            src,
            "t.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            None,
            false,
            Some(&mut cap),
            &[],
        )
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out), "clique\n3\n4\nesquerdo\n");
    }

    #[test]
    fn evento_zero_sem_fila_e_nada() {
        let src = r#"
importe "tela_grafica"
g = nova(8, 8)
ev = g.evento(0)
escreva(ev:tipo)
"#;
        let mut cap = CapturaGraf::default();
        let mut input = std::io::Cursor::new(String::new());
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_with_io(
            src,
            "t.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            None,
            false,
            Some(&mut cap),
            &[],
        )
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out), "nada\n");
    }
}
