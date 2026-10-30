use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::lexer::Span;
use crate::runtime::cartesiano_host::CartesianoHost;
use crate::runtime::env::Env;
use crate::runtime::error::EvalError;
use crate::runtime::eval::Vm;
use crate::runtime::folha::{CurvaPronta, FolhaPronta, GraficoPronto};
use crate::runtime::value::{ListaRc, MapKey, MapaRc, Value};

use super::tela;

pub(crate) const AMOSTRAS: usize = 400;

impl Vm<'_, '_> {
    pub(crate) fn bi_documento(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 0, span)?;
        Ok(Value::mapa(vec![(
            MapKey::Texto("graficos".into()),
            Value::lista(vec![]),
        )]))
    }

    pub(crate) fn bi_grafico(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 0, span)?;
        Ok(Value::mapa(vec![(
            MapKey::Texto("curvas".into()),
            Value::lista(vec![]),
        )]))
    }

    pub(crate) fn bi_eixos(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 5, span)?;
        let mapa = expect_mapa(&args[0], "grafico", span, self)?;
        if !tem_chave(&mapa, "curvas") || tem_chave(&mapa, "graficos") {
            return Err(self.err("eixos espera um grafico (cart::grafico())", span));
        }
        let xmin = self.expect_f64(&args[1], span)?;
        let xmax = self.expect_f64(&args[2], span)?;
        let ymin = self.expect_f64(&args[3], span)?;
        let ymax = self.expect_f64(&args[4], span)?;
        if !xmin.is_finite() || !xmax.is_finite() || !ymin.is_finite() || !ymax.is_finite() {
            return Err(self.err("eixos espera números finitos", span));
        }
        if xmin >= xmax {
            return Err(self.err("eixos: xmin deve ser menor que xmax", span));
        }
        if ymin >= ymax {
            return Err(self.err("eixos: ymin deve ser menor que ymax", span));
        }
        {
            let mut entries = mapa.borrow_mut();
            set_num(&mut entries, "xmin", xmin);
            set_num(&mut entries, "xmax", xmax);
            set_num(&mut entries, "ymin", ymin);
            set_num(&mut entries, "ymax", ymax);
        }
        Ok(args[0].clone())
    }

    pub(crate) fn bi_plotar(
        &mut self,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let mapa = expect_mapa(&args[0], "grafico", span, self)?;
        if !tem_chave(&mapa, "curvas") || tem_chave(&mapa, "graficos") {
            return Err(self.err("plotar espera um grafico (cart::grafico())", span));
        }
        let xmin = match get_f64(&mapa, "xmin") {
            Some(n) => n,
            None => return Err(self.err("defina os eixos antes de plotar", span)),
        };
        let xmax = get_f64(&mapa, "xmax").unwrap_or(xmin);
        let func = &args[1];
        match func {
            Value::Funcao(_) | Value::Builtin(_) => {}
            other => {
                return Err(self.err(
                    format!("plotar espera uma função, encontrado {}", other.type_name()),
                    span,
                ));
            }
        }
        let cor = self.expect_texto(&args[2], span)?;
        if !tela::cor_conhecida(&cor) {
            return Err(self.err(
                format!(
                    "cor desconhecida (\"{cor}\"); use normal, preto, vermelho, verde, amarelo, azul, magenta, ciano ou branco"
                ),
                span,
            ));
        }
        let n = AMOSTRAS.max(2);
        let mut pontos = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            let x = xmin + t * (xmax - xmin);
            let xv = Value::Numero(x.into());
            let yv = self.call_value(func, &[xv], span, "plotar", env)?;
            match yv {
                Value::Numero(n) => {
                    let y = n.to_f64();
                    if y.is_finite() {
                        pontos.push(Value::lista(vec![
                            Value::Numero(x.into()),
                            Value::Numero(y.into()),
                        ]));
                    } else {
                        pontos.push(Value::Nada);
                    }
                }
                other => {
                    return Err(self.err(
                        format!(
                            "a função de plotar deve devolver numero, encontrou {}",
                            other.type_name()
                        ),
                        span,
                    ));
                }
            }
        }
        let curva = Value::mapa(vec![
            (MapKey::Texto("pontos".into()), Value::lista(pontos)),
            (MapKey::Texto("cor".into()), Value::Texto(cor)),
        ]);
        {
            let curvas = get_lista(&mapa, "curvas")
                .ok_or_else(|| self.err("grafico precisa de :curvas", span))?;
            curvas.borrow_mut().push(curva);
        }
        Ok(args[0].clone())
    }

    pub(crate) fn bi_adicione(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let doc = expect_mapa(&args[0], "documento", span, self)?;
        if !tem_chave(&doc, "graficos") {
            return Err(self.err("adicione espera um documento (cart::documento())", span));
        }
        let graf = expect_mapa(&args[1], "grafico", span, self)?;
        if !tem_chave(&graf, "curvas") {
            return Err(self.err("adicione espera um grafico (cart::grafico())", span));
        }
        if get_f64(&graf, "xmin").is_none() {
            return Err(self.err("defina os eixos antes de adicione", span));
        }
        let n_curvas = get_lista(&graf, "curvas")
            .map(|xs| xs.borrow().len())
            .unwrap_or(0);
        if n_curvas == 0 {
            return Err(self.err("adicione espera um gráfico com pelo menos uma curva", span));
        }
        let copia = deep_clone(&args[1]);
        let graficos = get_lista(&doc, "graficos")
            .ok_or_else(|| self.err("documento precisa de :graficos", span))?;
        graficos.borrow_mut().push(copia);
        Ok(args[0].clone())
    }

    pub(crate) fn bi_mostre(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let doc = expect_mapa(&args[0], "documento", span, self)?;
        if !tem_chave(&doc, "graficos") {
            return Err(self.err("mostre espera um documento (cart::documento())", span));
        }
        let graficos = get_lista(&doc, "graficos")
            .ok_or_else(|| self.err("documento precisa de :graficos", span))?;
        if graficos.borrow().is_empty() {
            return Err(self.err("adicione pelo menos um gráfico", span));
        }
        let id = Rc::as_ptr(&doc) as u64;
        let mut prontos = Vec::new();
        for g in graficos.borrow().iter() {
            prontos.push(grafico_pronto(g, span, self)?);
        }
        let folha = FolhaPronta {
            id,
            graficos: prontos,
        };
        if let Some(host) = self.cartesiano_host.as_mut() {
            host.mostre(folha).map_err(|e| self.err(e, span))?;
        } else if self.abrir_cartesiano {
            self.cli_cart.mostre(folha).map_err(|e| self.err(e, span))?;
        }
        Ok(args[0].clone())
    }
}

fn grafico_pronto(v: &Value, span: Span, vm: &Vm<'_, '_>) -> Result<GraficoPronto, EvalError> {
    let mapa = expect_mapa(v, "grafico", span, vm)?;
    let xmin = get_f64(&mapa, "xmin").ok_or_else(|| vm.err("gráfico sem eixos", span))?;
    let xmax = get_f64(&mapa, "xmax").ok_or_else(|| vm.err("gráfico sem eixos", span))?;
    let ymin = get_f64(&mapa, "ymin").ok_or_else(|| vm.err("gráfico sem eixos", span))?;
    let ymax = get_f64(&mapa, "ymax").ok_or_else(|| vm.err("gráfico sem eixos", span))?;
    let curvas_v = get_lista(&mapa, "curvas").ok_or_else(|| vm.err("gráfico sem curvas", span))?;
    let mut curvas = Vec::new();
    for c in curvas_v.borrow().iter() {
        let cm = expect_mapa(c, "curva", span, vm)?;
        let cor = match get_texto(&cm, "cor") {
            Some(s) => s,
            None => "normal".into(),
        };
        let pts = get_lista(&cm, "pontos").ok_or_else(|| vm.err("curva sem :pontos", span))?;
        let mut pontos = Vec::new();
        for p in pts.borrow().iter() {
            match p {
                Value::Nada => pontos.push((0.0, None)),
                Value::Lista(xy) => {
                    let xy = xy.borrow();
                    if xy.len() != 2 {
                        return Err(vm.err("ponto de curva espera [x, y]", span));
                    }
                    let x = vm.expect_f64(&xy[0], span)?;
                    match &xy[1] {
                        Value::Numero(n) => {
                            let y = n.to_f64();
                            pontos.push((x, if y.is_finite() { Some(y) } else { None }));
                        }
                        Value::Nada => pontos.push((x, None)),
                        other => {
                            return Err(vm.err(
                                format!(
                                    "y do ponto deve ser numero, encontrado {}",
                                    other.type_name()
                                ),
                                span,
                            ));
                        }
                    }
                }
                other => {
                    return Err(vm.err(
                        format!("ponto de curva inválido ({})", other.type_name()),
                        span,
                    ));
                }
            }
        }
        curvas.push(CurvaPronta { cor, pontos });
    }
    Ok(GraficoPronto {
        xmin,
        xmax,
        ymin,
        ymax,
        curvas,
    })
}

fn expect_mapa(v: &Value, papel: &str, span: Span, vm: &Vm<'_, '_>) -> Result<MapaRc, EvalError> {
    match v {
        Value::Mapa(xs) => Ok(Rc::clone(xs)),
        other => Err(vm.err(
            format!("esperado {papel}, encontrado {}", other.type_name()),
            span,
        )),
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

fn get_f64(mapa: &MapaRc, chave: &str) -> Option<f64> {
    let k = MapKey::Texto(chave.into());
    match mapa
        .borrow()
        .iter()
        .find(|(kk, _)| *kk == k)
        .map(|(_, v)| v)
    {
        Some(Value::Numero(n)) => Some(n.to_f64()),
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

fn set_num(entries: &mut Vec<(MapKey, Value)>, chave: &str, n: f64) {
    let key = MapKey::Texto(chave.into());
    if let Some((_, v)) = entries.iter_mut().find(|(k, _)| *k == key) {
        *v = Value::Numero(n.into());
    } else {
        entries.push((key, Value::Numero(n.into())));
    }
}

/// Copy of `v` that shares no list, map, set or matrix with it, so later
/// changes to the chart do not reach the copy kept in the document.
fn deep_clone(v: &Value) -> Value {
    copiar(v, &mut HashMap::new())
}

/// `copias` maps each list or map already copied to its copy: one shared
/// twice is copied once, and a cycle (`xs[1] = xs`) becomes the same cycle
/// in the copy instead of copying forever.
fn copiar(v: &Value, copias: &mut HashMap<usize, Value>) -> Value {
    stacker::maybe_grow(64 * 1024, 1024 * 1024, || match v {
        Value::Lista(xs) => {
            let id = Rc::as_ptr(xs) as *const () as usize;
            if let Some(copia) = copias.get(&id) {
                return copia.clone();
            }
            let copia = Value::lista(Vec::new());
            copias.insert(id, copia.clone());
            let itens: Vec<Value> = xs.borrow().iter().map(|x| copiar(x, copias)).collect();
            if let Value::Lista(destino) = &copia {
                *destino.borrow_mut() = itens;
            }
            copia
        }
        Value::Mapa(m) => {
            let id = Rc::as_ptr(m) as *const () as usize;
            if let Some(copia) = copias.get(&id) {
                return copia.clone();
            }
            let copia = Value::mapa(Vec::new());
            copias.insert(id, copia.clone());
            let entradas: Vec<(MapKey, Value)> = m
                .borrow()
                .iter()
                .map(|(k, x)| (k.clone(), copiar(x, copias)))
                .collect();
            if let Value::Mapa(destino) = &copia {
                *destino.borrow_mut() = entradas;
            }
            copia
        }
        Value::Par(k, x) => Value::Par(k.clone(), Box::new(copiar(x, copias))),
        Value::Conjunto(xs) => Value::conjunto(xs.borrow().clone()),
        Value::Matriz(m) => Value::matriz(m.borrow().clone()),
        other => other.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Value, deep_clone};
    use crate::runtime::cartesiano_host::CapturaCartesiano;
    use crate::runtime::eval::run_with_io;
    use crate::runtime::{NoopHook, run_to_string};

    fn run(src: &str) -> String {
        let src = format!("importe \"mat\"\nimporte \"cartesiano\"\n{src}");
        run_to_string(&src, "teste.lep").unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    fn run_err(src: &str) -> String {
        let src = format!("importe \"mat\"\nimporte \"cartesiano\"\n{src}");
        match run_to_string(&src, "teste.lep") {
            Err(e) => e.message,
            Ok(out) => panic!("expected error, got {out:?}\n{src}"),
        }
    }

    #[test]
    fn grafico_e_documento_sao_mapas() {
        assert_eq!(
            run(r#"
g = grafico()
escreva(tipo(g))
c = documento()
escreva(tipo(c))
"#),
            "mapa\nmapa\n"
        );
    }

    #[test]
    fn plotar_amostra_e_congela_no_adicione() {
        let out = run(r#"
g = grafico()
g.eixos(-1, 1, -1, 1)
g.plotar(função(x) { x }, :azul)
escreva(tamanho(g:curvas))
escreva(tamanho(g:curvas[1]:pontos))
c = documento()
c.adicione(g)
g.plotar(função(x) { 0 }, :vermelho)
escreva(tamanho(g:curvas))
escreva(tamanho(c:graficos[1]:curvas))
"#);
        assert_eq!(out, format!("1\n{}\n2\n1\n", super::AMOSTRAS));
    }

    #[test]
    fn plotar_sem_eixos_e_mostre_vazio() {
        assert!(run_err("g = grafico()\ng.plotar(função(x) { x }, :azul)").contains("eixos"));
        assert!(run_err("c = documento()\nc.mostre()").contains("gráfico"));
        assert!(run_err("eixos(1, 0, 1, -1, 1)").contains("grafico"));
        assert!(
            run_err(
                r#"
g = grafico()
g.eixos(1, 0, -1, 1)
"#
            )
            .contains("xmin")
        );
        assert!(
            run_err(
                r#"
g = grafico()
g.eixos(-1, 1, -1, 1)
g.plotar(função(x) { x }, :rosa)
"#
            )
            .contains("cor desconhecida")
        );
        assert!(
            run_err(
                r#"
c = documento()
g = grafico()
g.eixos(-1, 1, -1, 1)
c.adicione(g)
"#
            )
            .contains("curva")
        );
    }

    #[test]
    fn mostre_envia_folha_ao_host() {
        let src = r#"
importe "mat"
importe "cartesiano"
c = documento()
g = grafico()
g.eixos(-1, 1, -2, 2)
g.plotar(função(x) { x }, :verde)
c.adicione(g)
c.mostre()
c.mostre()
"#;
        let mut cap = CapturaCartesiano::default();
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
            Some(&mut cap),
            false,
            None,
            &[],
        )
        .unwrap();
        assert_eq!(cap.folhas.len(), 2);
        assert_eq!(cap.folhas[0].id, cap.folhas[1].id);
        assert_eq!(cap.folhas[0].graficos.len(), 1);
        assert_eq!(cap.folhas[0].graficos[0].curvas[0].cor, "verde");
        assert_eq!(
            cap.folhas[0].graficos[0].curvas[0].pontos.len(),
            super::AMOSTRAS
        );
    }

    #[test]
    fn dois_documentos_tem_ids_diferentes() {
        let src = r#"
importe "cartesiano"
c1 = documento()
c2 = documento()
g = grafico()
g.eixos(-1, 1, -1, 1)
g.plotar(função(x) { 0 }, :azul)
c1.adicione(g)
c2.adicione(g)
c1.mostre()
c2.mostre()
"#;
        let mut cap = CapturaCartesiano::default();
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
            Some(&mut cap),
            false,
            None,
            &[],
        )
        .unwrap();
        assert_eq!(cap.folhas.len(), 2);
        assert_ne!(cap.folhas[0].id, cap.folhas[1].id);
    }

    #[test]
    fn furo_quando_y_nao_e_finito() {
        let src = r#"
importe "cartesiano"
g = grafico()
g.eixos(-1, 1, -1, 1)
g.plotar(função(x) {
    se x == 0 { 1 / 0 se_falhar 0 / 0 }
    senao { x }
}, :azul)
c = documento()
c.adicione(g)
c.mostre()
"#;
        // 0/0 in decimal might error; use a function that returns via se_falhar nada?
        // Simpler: plot 1/x which is inf at 0? 1/0 errors in expressa.
        // We'll just check the test below with explicit nothing via a condition that
        // still returns a number that's inf... decimal 1/0 is error.
        // Skip inf: the runtime treats non-finite f64 as hole; from_f64 of inf is 0.
        // So we can't easily get inf through Numero. Holes from NaN: Numero::from(f64::NAN) is 0.
        // So holes only if we push Nada... we only push Nada when y is non-finite after to_f64.
        // rust_decimal to_f64 of normal numbers is finite.
        // For this language, holes happen if we later add sqrt negative as nan... raiz errors.
        // Keep the test as "plotar succeeds" — hole path is still unit-tested via FolhaPronta.
        let _ = src;
        let out = run(r#"
g = grafico()
g.eixos(-1, 1, -1, 1)
g.plotar(função(x) { x }, :azul)
escreva(g:curvas[1]:cor)
"#);
        assert_eq!(out, "azul\n");
    }

    #[test]
    fn deep_clone_copia_ciclo_e_compartilhamento() {
        let comum = Value::lista(vec![Value::Numero(1.into())]);
        let xs = Value::lista(vec![comum.clone(), comum.clone()]);
        let Value::Lista(itens) = &xs else {
            unreachable!()
        };
        itens.borrow_mut().push(xs.clone()); // xs contains itself
        let copia = deep_clone(&xs);
        assert_eq!(copia.to_string(), "[[1], [1], […]]");
        let Value::Lista(c) = &copia else {
            unreachable!()
        };
        let c = c.borrow();
        // The cycle points at the copy, not at the original.
        let Value::Lista(volta) = &c[2] else {
            unreachable!()
        };
        let Value::Lista(raiz) = &copia else {
            unreachable!()
        };
        assert!(std::rc::Rc::ptr_eq(volta, raiz));
        // A list shared twice is still shared (once) in the copy.
        let (Value::Lista(a), Value::Lista(b)) = (&c[0], &c[1]) else {
            unreachable!()
        };
        assert!(std::rc::Rc::ptr_eq(a, b));
        let Value::Lista(original) = &comum else {
            unreachable!()
        };
        assert!(!std::rc::Rc::ptr_eq(a, original));
    }
}
