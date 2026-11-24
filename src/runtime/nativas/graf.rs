use std::rc::Rc;

use super::super::fonte8x8;
use crate::lexer::Span;
use crate::runtime::error::EvalError;
use crate::runtime::eval::Vm;
use crate::runtime::graf::{
    ALTURA_MAX, LARGURA_MAX, LIMITE_COORD, OpGraf, TAMANHO_PIXEL_MAX, TelaGrafica,
    cor_graf_conhecida,
};
use crate::runtime::graf_host::{CapturaEvento, EventoGraf, GrafHost};
use crate::runtime::imagem::{self, Imagem};
use crate::runtime::value::{ListaRc, MapKey, MapaRc, Value};

impl Vm<'_, '_> {
    pub(crate) fn bi_graf_cores(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 0, span)?;
        Ok(Value::lista(
            crate::runtime::graf::PALETA
                .iter()
                .map(|nome| Value::Texto((*nome).into()))
                .collect(),
        ))
    }

    pub(crate) fn bi_graf_rgb(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let mut componentes = [0u8; 3];
        for (i, arg) in args.iter().enumerate() {
            let n = self.expect_int(arg, span)?;
            componentes[i] = u8::try_from(n)
                .map_err(|_| self.err("componente RGB deve ser um inteiro de 0 a 255", span))?;
        }
        Ok(Value::Texto(format!(
            "#{:02X}{:02X}{:02X}",
            componentes[0], componentes[1], componentes[2]
        )))
    }

    pub(crate) fn bi_graf_nova(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.len() != 2 && args.len() != 3 {
            return Err(self.err(
                format!("esperado 2 ou 3 argumento(s), recebeu {}", args.len()),
                span,
            ));
        }
        let largura = self.expect_graf_dim(&args[0], "largura", span)?;
        let altura = self.expect_graf_dim(&args[1], "altura", span)?;
        let tamanho_pixel = if args.len() == 3 {
            self.expect_tamanho_pixel(&args[2], span)?
        } else {
            1
        };
        Ok(Value::mapa_recurso(vec![
            (
                MapKey::Texto("largura".into()),
                Value::Numero(largura.into()),
            ),
            (MapKey::Texto("altura".into()), Value::Numero(altura.into())),
            (
                MapKey::Texto("tamanho_pixel".into()),
                Value::Numero(tamanho_pixel.into()),
            ),
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
                    Value::Numero((x1 as i64).into()),
                ),
                (
                    MapKey::Texto("y1".into()),
                    Value::Numero((y1 as i64).into()),
                ),
                (
                    MapKey::Texto("x2".into()),
                    Value::Numero((x2 as i64).into()),
                ),
                (
                    MapKey::Texto("y2".into()),
                    Value::Numero((y2 as i64).into()),
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
        let r = self.expect_raio(&args[3], span)?;
        let cor = pen(&mapa);
        push_op(
            &mapa,
            Value::mapa(vec![
                (MapKey::Texto("tipo".into()), Value::Texto("circulo".into())),
                (MapKey::Texto("x".into()), Value::Numero((x as i64).into())),
                (MapKey::Texto("y".into()), Value::Numero((y as i64).into())),
                (MapKey::Texto("r".into()), Value::Numero((r as i64).into())),
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
                (MapKey::Texto("x".into()), Value::Numero((x as i64).into())),
                (MapKey::Texto("y".into()), Value::Numero((y as i64).into())),
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
                    Value::Numero((x1 as i64).into()),
                ),
                (
                    MapKey::Texto("y1".into()),
                    Value::Numero((y1 as i64).into()),
                ),
                (
                    MapKey::Texto("x2".into()),
                    Value::Numero((x2 as i64).into()),
                ),
                (
                    MapKey::Texto("y2".into()),
                    Value::Numero((y2 as i64).into()),
                ),
                (MapKey::Texto("cor".into()), Value::Texto(cor)),
            ]),
        );
        Ok(args[0].clone())
    }

    pub(crate) fn bi_graf_texto(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if !(4..=6).contains(&args.len()) {
            return Err(self.err(
                format!("esperado 4, 5 ou 6 argumento(s), recebeu {}", args.len()),
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
                (MapKey::Texto("x".into()), Value::Numero((x as i64).into())),
                (MapKey::Texto("y".into()), Value::Numero((y as i64).into())),
                (
                    MapKey::Texto("tamanho".into()),
                    Value::Numero((tamanho as i64).into()),
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
        let (eixo, angulo) = match args.get(4) {
            Some(opcoes) => opcoes_imagem(opcoes, span, self)?,
            None => ([0.0, 0.0], 0.0),
        };
        push_op(
            &mapa,
            Value::mapa(vec![
                (MapKey::Texto("tipo".into()), Value::Texto("imagem".into())),
                (MapKey::Texto("figura".into()), Value::imagem(figura)),
                (
                    MapKey::Texto("eixo".into()),
                    Value::mapa(vec![
                        (
                            MapKey::Texto("x".into()),
                            self.number_from_f64(eixo[0], span)?,
                        ),
                        (
                            MapKey::Texto("y".into()),
                            self.number_from_f64(eixo[1], span)?,
                        ),
                    ]),
                ),
                (MapKey::Texto("x".into()), Value::Numero((x as i64).into())),
                (MapKey::Texto("y".into()), Value::Numero((y as i64).into())),
                (
                    MapKey::Texto("angulo".into()),
                    self.number_from_f64(angulo, span)?,
                ),
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
        Ok(Value::imagem(tela.como_imagem()))
    }

    pub(crate) fn bi_graf_mostre(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let tela = tela_de(&args[0], span, self)?;
        let id = tela.id;
        // Include a host that creates its window and then fails presenting it.
        if self.graf_host.is_some() || self.abrir_cartesiano {
            self.janelas_graficas.insert(id);
        }
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
        self.expect_arity(args, 1, span)?;
        self.check_deadline(span)?;
        if self.interrupted() {
            return Err(EvalError::Quit(0));
        }
        let tela = tela_para_evento(&args[0], span, self)?;
        let captura = if let Some(host) = self.graf_host.as_mut() {
            host.evento(&tela)
        } else if self.abrir_cartesiano {
            self.cli_graf.evento(&tela)
        } else {
            Err("evento() precisa de uma janela gráfica".into())
        }
        .map_err(|e| self.err(e, span))?;
        Ok(captura_valor(captura))
    }

    fn expect_tamanho_pixel(&self, v: &Value, span: Span) -> Result<u32, EvalError> {
        let n = self.expect_f64(v, span)?;
        if !n.is_finite() || n < 1.0 || n.fract() != 0.0 || n > f64::from(TAMANHO_PIXEL_MAX) {
            return Err(self.err(
                format!("tamanho_pixel deve ser um inteiro de 1 a {TAMANHO_PIXEL_MAX}"),
                span,
            ));
        }
        Ok(n as u32)
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
        limitar_coord(n).ok_or_else(|| {
            self.err(
                format!("coordenada fora do intervalo (entre -{LIMITE_COORD} e {LIMITE_COORD})"),
                span,
            )
        })
    }

    fn expect_raio(&self, v: &Value, span: Span) -> Result<i32, EvalError> {
        let n = self.expect_f64(v, span)?;
        if !n.is_finite() {
            return Err(self.err("raio deve ser um número finito", span));
        }
        let arred = n.round();
        if arred < 0.0 {
            return Err(self.err("raio do círculo deve ser ≥ 0", span));
        }
        if arred > f64::from(LIMITE_COORD) {
            return Err(self.err(
                format!("raio do círculo deve ser um inteiro de 0 a {LIMITE_COORD}"),
                span,
            ));
        }
        Ok(arred as i32)
    }
}

fn numero_finito_imagem(
    v: &Value,
    campo: &str,
    span: Span,
    vm: &Vm<'_, '_>,
) -> Result<f64, EvalError> {
    let n = vm.expect_f64(v, span)?;
    if !n.is_finite() {
        return Err(vm.err(format!("{campo} da imagem deve ser um número finito"), span));
    }
    Ok(n)
}

fn eixo_imagem(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<[f64; 2], EvalError> {
    let Value::Mapa(m) = v else {
        return Err(vm.err("eixo da imagem deve ser um mapa com :x e :y", span));
    };
    let m = m.borrow();
    if m.keys()
        .any(|k| !matches!(k, MapKey::Texto(s) if s == "x" || s == "y"))
    {
        return Err(vm.err("eixo da imagem aceita somente :x e :y", span));
    }
    let mut eixo = [0.0; 2];
    for (i, campo) in ["x", "y"].into_iter().enumerate() {
        let v = m
            .get(&MapKey::Texto(campo.into()))
            .ok_or_else(|| vm.err(format!("eixo da imagem exige :{campo}"), span))?;
        eixo[i] = numero_finito_imagem(v, campo, span, vm)?;
    }
    Ok(eixo)
}

fn opcoes_imagem(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<([f64; 2], f64), EvalError> {
    let Value::Mapa(m) = v else {
        return Err(vm.err(
            "opções da imagem devem ser um mapa com :eixo e/ou :angulo",
            span,
        ));
    };
    let mut eixo = [0.0; 2];
    let mut angulo = 0.0;
    for (chave, valor) in m.borrow().iter() {
        match chave {
            MapKey::Texto(s) if s == "eixo" => eixo = eixo_imagem(valor, span, vm)?,
            MapKey::Texto(s) if s == "angulo" => {
                angulo = numero_finito_imagem(valor, "ângulo", span, vm)?
            }
            _ => return Err(vm.err("opção de imagem desconhecida; use :eixo e :angulo", span)),
        }
    }
    Ok((eixo, angulo))
}

fn limitar_coord(n: f64) -> Option<i32> {
    let arred = n.round();
    if arred < f64::from(-LIMITE_COORD) || arred > f64::from(LIMITE_COORD) {
        return None;
    }
    Some(arred as i32)
}

pub(crate) fn is_graf(v: &Value) -> bool {
    let Value::Mapa(xs) = v else {
        return false;
    };
    tem_chave(xs, "largura") && tem_chave(xs, "altura") && tem_chave(xs, "ops")
}

pub(super) fn expect_graf(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<MapaRc, EvalError> {
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
    let tamanho_pixel = tamanho_pixel_do_mapa(&mapa, span, vm)?;
    Ok(TelaGrafica {
        id: Rc::as_ptr(xs) as u64,
        largura,
        altura,
        tamanho_pixel,
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
    let tamanho_pixel = tamanho_pixel_do_mapa(&mapa, span, vm)?;
    let ops_v = get_lista(&mapa, "ops").ok_or_else(|| vm.err("tela gráfica sem :ops", span))?;
    let mut ops = Vec::new();
    for item in ops_v.borrow().iter() {
        ops.push(op_de(item, span, vm)?);
    }
    Ok(TelaGrafica {
        id: Rc::as_ptr(xs) as u64,
        largura,
        altura,
        tamanho_pixel,
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
            x: exigir_coord(xs, "x", span, vm)?,
            y: exigir_coord(xs, "y", span, vm)?,
            cor,
        }),
        "linha" => Ok(OpGraf::Linha {
            x1: exigir_coord(xs, "x1", span, vm)?,
            y1: exigir_coord(xs, "y1", span, vm)?,
            x2: exigir_coord(xs, "x2", span, vm)?,
            y2: exigir_coord(xs, "y2", span, vm)?,
            cor,
        }),
        "circulo" => Ok(OpGraf::Circulo {
            x: exigir_coord(xs, "x", span, vm)?,
            y: exigir_coord(xs, "y", span, vm)?,
            r: exigir_raio(xs, span, vm)?,
            cor,
        }),
        "imagem" => {
            let figura = get_imagem(xs, "figura")
                .ok_or_else(|| vm.err("comando de imagem sem :figura", span))?;
            let eixo = match xs.borrow().get(&MapKey::Texto("eixo".into())) {
                Some(v) => eixo_imagem(v, span, vm)?,
                None => [0.0, 0.0],
            };
            Ok(OpGraf::Imagem {
                eixo,
                x: exigir_coord(xs, "x", span, vm)?,
                y: exigir_coord(xs, "y", span, vm)?,
                angulo: get_f64(xs, "angulo").unwrap_or(0.0),
                figura,
            })
        }
        "texto" => Ok(OpGraf::Texto {
            x: exigir_coord(xs, "x", span, vm)?,
            y: exigir_coord(xs, "y", span, vm)?,
            tamanho: get_i32(xs, "tamanho")
                .unwrap_or(fonte8x8::TAMANHO_PADRAO)
                .clamp(1, fonte8x8::TAMANHO_MAX),
            cor,
            texto: get_texto(xs, "texto").unwrap_or_default(),
        }),
        "retangulo" | "retângulo" => Ok(OpGraf::Retangulo {
            x1: exigir_coord(xs, "x1", span, vm)?,
            y1: exigir_coord(xs, "y1", span, vm)?,
            x2: exigir_coord(xs, "x2", span, vm)?,
            y2: exigir_coord(xs, "y2", span, vm)?,
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
                "cor desconhecida (\"{cor}\"); use cores() para listar os nomes ou rgb(vermelho, verde, azul) para criar uma cor #RRGGBB"
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

fn tamanho_pixel_do_mapa(mapa: &MapaRc, span: Span, vm: &Vm<'_, '_>) -> Result<u32, EvalError> {
    if !tem_chave(mapa, "tamanho_pixel") {
        return Ok(1);
    }
    let k = MapKey::Texto("tamanho_pixel".into());
    match mapa.borrow().get(&k) {
        Some(Value::Numero(n)) => {
            let f = n.to_f64();
            if f.is_finite() && f >= 1.0 && f.fract() == 0.0 && f <= f64::from(TAMANHO_PIXEL_MAX) {
                Ok(f as u32)
            } else {
                Err(vm.err(
                    format!("tamanho_pixel deve ser um inteiro de 1 a {TAMANHO_PIXEL_MAX}"),
                    span,
                ))
            }
        }
        _ => Err(vm.err(
            format!("tamanho_pixel deve ser um inteiro de 1 a {TAMANHO_PIXEL_MAX}"),
            span,
        )),
    }
}

fn tem_chave(mapa: &MapaRc, chave: &str) -> bool {
    let k = MapKey::Texto(chave.into());
    mapa.borrow().contains_key(&k)
}

fn get_lista(mapa: &MapaRc, chave: &str) -> Option<ListaRc> {
    let k = MapKey::Texto(chave.into());
    match mapa.borrow().get(&k) {
        Some(Value::Lista(xs)) => Some(Rc::clone(xs)),
        _ => None,
    }
}

fn figura_de(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<Imagem, EvalError> {
    match v {
        Value::Imagem(img) => Ok(img.borrow().clone()),
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
    match mapa.borrow().get(&k) {
        Some(Value::Imagem(img)) => Some(img.borrow().clone()),
        _ => None,
    }
}

fn get_texto(mapa: &MapaRc, chave: &str) -> Option<String> {
    let k = MapKey::Texto(chave.into());
    match mapa.borrow().get(&k) {
        Some(Value::Texto(s)) => Some(s.clone()),
        _ => None,
    }
}

fn get_u32(mapa: &MapaRc, chave: &str) -> Option<u32> {
    let k = MapKey::Texto(chave.into());
    match mapa.borrow().get(&k) {
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
    match mapa.borrow().get(&k) {
        Some(Value::Numero(n)) => {
            let f = n.to_f64();
            f.is_finite().then_some(f)
        }
        _ => None,
    }
}

fn get_i32(mapa: &MapaRc, chave: &str) -> Option<i32> {
    match numero_campo(mapa, chave) {
        NumeroCampo::Ok(n) => Some(n.round() as i32),
        NumeroCampo::Ausente | NumeroCampo::NaoFinito => None,
    }
}

enum NumeroCampo {
    Ausente,
    NaoFinito,
    Ok(f64),
}

fn numero_campo(mapa: &MapaRc, chave: &str) -> NumeroCampo {
    let k = MapKey::Texto(chave.into());
    match mapa.borrow().get(&k) {
        Some(Value::Numero(n)) => {
            let f = n.to_f64();
            if f.is_finite() {
                NumeroCampo::Ok(f)
            } else {
                NumeroCampo::NaoFinito
            }
        }
        _ => NumeroCampo::Ausente,
    }
}

fn exigir_coord(mapa: &MapaRc, chave: &str, span: Span, vm: &Vm<'_, '_>) -> Result<i32, EvalError> {
    match numero_campo(mapa, chave) {
        NumeroCampo::Ausente => Ok(0),
        NumeroCampo::NaoFinito => Err(vm.err("coordenada deve ser um número finito", span)),
        NumeroCampo::Ok(n) => limitar_coord(n).ok_or_else(|| {
            vm.err(
                format!("coordenada fora do intervalo (entre -{LIMITE_COORD} e {LIMITE_COORD})"),
                span,
            )
        }),
    }
}

fn exigir_raio(mapa: &MapaRc, span: Span, vm: &Vm<'_, '_>) -> Result<i32, EvalError> {
    match numero_campo(mapa, "r") {
        NumeroCampo::Ausente => Ok(0),
        NumeroCampo::NaoFinito => Err(vm.err("raio deve ser um número finito", span)),
        NumeroCampo::Ok(n) => {
            let arred = n.round();
            if arred < 0.0 || arred > f64::from(LIMITE_COORD) {
                Err(vm.err(
                    format!("raio do círculo deve ser um inteiro de 0 a {LIMITE_COORD}"),
                    span,
                ))
            } else {
                Ok(arred as i32)
            }
        }
    }
}

fn evento_valor(e: EventoGraf) -> Value {
    Value::mapa(vec![
        (MapKey::Texto("tipo".into()), Value::Texto(e.tipo)),
        (
            MapKey::Texto("x".into()),
            Value::Numero((e.x as i64).into()),
        ),
        (
            MapKey::Texto("y".into()),
            Value::Numero((e.y as i64).into()),
        ),
        (MapKey::Texto("botao".into()), Value::Texto(e.botao)),
        (MapKey::Texto("tecla".into()), Value::Texto(e.tecla)),
        (
            MapKey::Texto("modificadores".into()),
            conjunto_valor(e.modificadores),
        ),
        (MapKey::Texto("repeticao".into()), Value::Bool(e.repeticao)),
    ])
}

fn conjunto_valor(teclas: std::collections::BTreeSet<String>) -> Value {
    Value::conjunto_de_hash(teclas.into_iter().map(MapKey::Texto).collect())
}

fn captura_valor(e: CapturaEvento) -> Value {
    Value::mapa(vec![
        (MapKey::Texto("teclas".into()), conjunto_valor(e.teclas)),
        (
            MapKey::Texto("apertadas".into()),
            conjunto_valor(e.apertadas),
        ),
        (MapKey::Texto("soltas".into()), conjunto_valor(e.soltas)),
        (
            MapKey::Texto("eventos".into()),
            Value::lista(e.eventos.into_iter().map(evento_valor).collect()),
        ),
        (MapKey::Texto("sair".into()), Value::Bool(e.sair)),
    ])
}

fn set_texto(entries: &mut std::collections::HashMap<MapKey, Value>, chave: &str, s: String) {
    let key = MapKey::Texto(chave.into());
    entries.insert(key, Value::Texto(s));
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

    fn nos_dois(src: &str) -> String {
        use crate::runtime::{ExecutionBackend, ExecutionOptions, run_with_options};
        let src = format!("importe \"tela_grafica\"\n{src}");
        let mut visto = None;
        for backend in [ExecutionBackend::Bytecode, ExecutionBackend::Ast] {
            if !backend.is_available() {
                continue;
            }
            let saiu = run_with_options(
                &src,
                "teste.lep",
                "",
                ExecutionOptions {
                    backend,
                    ..ExecutionOptions::default()
                },
            )
            .map(|o| o.stdout)
            .unwrap_or_else(|e| panic!("{backend:?}: {e}\n{src}"));
            if let Some(antes) = &visto {
                assert_eq!(&saiu, antes, "{backend:?}");
            }
            visto = Some(saiu);
        }
        visto.expect("nenhum backend")
    }

    #[test]
    fn nova_e_mapa_e_pixel_do_fundo() {
        assert_eq!(
            run(r#"
g = nova(16, 16)
escreva(tipo(g))
escreva(g.pixel(0, 0))
escreva(g:tamanho_pixel)
"#),
            "mapa\nbranco\n1\n"
        );
    }

    #[test]
    fn nova_tamanho_pixel_fica_no_mapa_e_nao_no_canvas() {
        assert_eq!(
            run(r#"
g = nova(160, 144, 4)
escreva(g:largura)
escreva(g:altura)
escreva(g:tamanho_pixel)
g.cor(:azul)
g.ponto(0, 0)
escreva(g.pixel(0, 0))
img = g.como_imagem()
escreva(tipo(img))
"#),
            "160\n144\n4\nazul\nimagem\n"
        );
        assert!(run_err("g = nova(8, 8, 0)\n").contains("tamanho_pixel"));
        assert!(run_err("g = nova(8, 8, 17)\n").contains("tamanho_pixel"));
        assert!(run_err("g = nova(8, 8, 1.5)\n").contains("tamanho_pixel"));
        assert!(run_err("g = nova(8)\n").contains("2 ou 3"));
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
    fn coordenada_e_raio_enormes_sao_erro_e_o_recorte_fica() {
        let fora = run_err("g = nova(10, 10)\ng.linha(-9999999999, 0, 9999999999, 5)\n");
        assert!(fora.contains("coordenada fora do intervalo"), "{fora}");
        let raio = run_err("g = nova(10, 10)\ng.circulo(0, 0, 99999999999)\n");
        assert!(raio.contains("raio do círculo"), "{raio}");
        assert!(run_err("g = nova(10, 10)\ng.circulo(0, 0, -1)\n").contains("≥ 0"));
        assert_eq!(
            nos_dois(
                r#"
g = nova(10, 10)
g.cor(:azul)
escreva(g.linha(-9999999999, 0, 9999999999, 5) se_falhar { "e" })
escreva(g.circulo(0, 0, 99999999999) se_falhar { "e" })
g.linha(-20, 1, 30, 1)
escreva(g.pixel(0, 1))
escreva(g.pixel(9, 1))
escreva(g.pixel(0, 0))
g.circulo(5, 5, 3)
escreva(g.pixel(8, 5))
"#
            ),
            "e\ne\nazul\nazul\nbranco\nazul\n"
        );
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
        assert_eq!(out, "transparente\nverde\nimagem\ntransparente\nazul\n");
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
w.imagem(celula, 2, 2)
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
        assert_eq!(
            rgb[(3 * 8 + 3) as usize],
            0x0022AA44,
            "célula verde no centro"
        );
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
        assert_eq!(cap.telas[0].tamanho_pixel, 1);
        assert_eq!(cap.telas[0].janela_largura(), 8);
    }

    #[test]
    fn mostre_tamanho_pixel_abre_janela_maior() {
        let src = r#"
importe "tela_grafica"
g = nova(160, 144, 4)
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
        assert_eq!(cap.telas[0].largura, 160);
        assert_eq!(cap.telas[0].altura, 144);
        assert_eq!(cap.telas[0].tamanho_pixel, 4);
        assert_eq!(cap.telas[0].janela_largura(), 640);
        assert_eq!(cap.telas[0].janela_altura(), 576);
        assert_eq!(cap.telas[0].rgb32().len(), 160 * 144);
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
g.imagem("sprite.png", 7, 7)
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
        assert!(run_err("g = nova(8, 8)\nescreva(g.evento())\n").contains("janela"));
    }

    #[test]
    fn evento_devolve_clique_da_fila() {
        let src = r#"
importe "tela_grafica"
g = nova(8, 8)
g.mostre()
ev = g.evento()
escreva(ev:eventos[0]:tipo)
escreva(ev:eventos[0]:x)
escreva(ev:eventos[0]:y)
escreva(ev:eventos[0]:botao)
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
    fn evento_sem_fila_devolve_captura_vazia() {
        let src = r#"
importe "tela_grafica"
g = nova(8, 8)
ev = g.evento()
escreva(tamanho(ev:teclas))
escreva(tamanho(ev:apertadas))
escreva(tamanho(ev:soltas))
escreva(tamanho(ev:eventos))
escreva(ev:sair)
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
        assert_eq!(String::from_utf8_lossy(&out), "0\n0\n0\n0\nfalso\n");
    }
    #[test]
    fn evento_exposes_sets_consumes_all_transitions_and_returns_independent_values() {
        let src = r#"
importe "tela_grafica"
g = nova(8, 8)
ev = g.evento()
escreva(tipo(ev:teclas))
escreva(ev:teclas contem :direita)
escreva(ev:apertadas contem :espaco)
escreva(ev:soltas contem :espaco)
escreva(ev:eventos[1]:modificadores contem :control)
// Reading fields twice must not consume them.
escreva(ev:apertadas contem :espaco)
ev:teclas = conjunto()
proximo = g.evento()
escreva(proximo:teclas contem :direita)
escreva(tamanho(proximo:apertadas))
escreva(tamanho(proximo:soltas))
escreva(tamanho(proximo:eventos))
"#;
        let mut cap = CapturaGraf::default();
        cap.eventos.extend([
            EventoGraf::tecla("control"),
            EventoGraf::tecla("direita"),
            EventoGraf::tecla("espaco"),
            EventoGraf::soltura("espaco"),
            EventoGraf::soltura("control"),
        ]);
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
        assert_eq!(
            String::from_utf8_lossy(&out),
            "conjunto\nverdadeiro\nverdadeiro\nverdadeiro\nverdadeiro\nverdadeiro\nverdadeiro\n0\n0\n0\n"
        );
    }
}
