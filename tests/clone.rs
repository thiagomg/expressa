use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, compile_exb, run_exb_with_options, run_with_options,
};
fn options(backend: ExecutionBackend) -> ExecutionOptions {
    ExecutionOptions {
        backend,
        ..Default::default()
    }
}
fn check(source: &str, expected: &str) {
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|b| b.is_available())
    {
        let out = run_with_options(source, "clone.lep", "", options(backend))
            .unwrap_or_else(|e| panic!("{backend:?}: {e}"));
        assert_eq!(out.stdout, expected, "{backend:?}");
        assert_eq!(out.exit_code, 0);
    }
    let bytes = compile_exb(source, "clone.lep").unwrap();
    let out =
        run_exb_with_options(&bytes, "clone.exb", "", options(ExecutionBackend::Bytecode)).unwrap();
    assert_eq!(out.stdout, expected, "EXB");
}
fn error(source: &str, part: &str) {
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|b| b.is_available())
    {
        let err = run_with_options(source, "clone.lep", "", options(backend)).unwrap_err();
        assert!(err.message.contains(part), "{backend:?}: {err}");
        assert_eq!(err.file, "clone.lep");
    }
    let bytes = compile_exb(source, "clone.lep").unwrap();
    let err = run_exb_with_options(&bytes, "clone.exb", "", options(ExecutionBackend::Bytecode))
        .unwrap_err();
    assert!(err.message.contains(part), "EXB: {err}");
}
#[test]
fn copies_nested_graph_preserving_aliases_and_cycles() {
    check(
        r#"
xs = [1]
a = mapa()
a:xs = xs
a:ys = xs
a:self = a
b = clone(a)
b:xs[0] = 9
escreva(a:xs[0], b:ys[0], b:self:xs[0])
l = [0]
l[0] = l
c = clone(l)
c[0][0] = 7
escreva(c[0], tipo(l[0]))
"#,
        "1 9 9\n7 lista\n",
    );
}
#[test]
fn copies_sets_matrices_pairs_and_keeps_text_value_semantics() {
    check(
        r#"
importe "matriz"
s = conjunto([1, 2])
m = zeros(2)
a = [s, s, m, m]
b = clone(a)
b[0] = remova(b[0], 1)
b[2][0, 0] = 9
escreva(tamanho(s), tamanho(b[1]), m[0, 0], b[3][0, 0])
p = :x -> [[1]]
q = clone(p)
q:valor[0][0] = 5
escreva(p:valor[0][0], q:valor[0][0])
t = ["oi"]
u = t
v = clone(t)
u[0][0] = "O"
escreva(t[0], v[0])
vazia = funcao() { se falso { 0 } }
escreva(clone(7), clone(verdadeiro), clone(vazia()), clone("abc"))
"#,
        "2 2 0 9\n1 5\nOi oi\n7 verdadeiro nada abc\n",
    );
}
#[test]
fn cloneable_marker_is_recursive_and_does_not_run_callbacks() {
    check(
        r#"
a = mapa()
a:_clonavel = falso
escreva(clone(a) se_falhar "bloqueado")
a:_clonavel = verdadeiro
escreva(tipo(clone(a)))
"#,
        "bloqueado\nmapa\n",
    );
    error("a = mapa()\na:_clonavel = falso\nclone([1, a])", "[1]");
    error("a = mapa()\na:_clonavel = 1\nclone(a)", "deve ser bool");
    error("clone()", "argumento");
}
#[test]
fn functions_without_captures_are_detached_and_keep_code_behavior() {
    check(
        r#"
f = funcao(x) { y = x + 1 y * 2 }
g = clone(f)
f = 0
escreva(g(3))
h = clone(funcao(x) { escreva(tipo(x)) x })
h(2)
factory = clone(funcao(x) { funcao(y) { x + y } })
escreva(factory(4)(5))
xs = [g, g]
ys = clone(xs)
escreva(ys[0] == ys[1], ys[0] == g)
escreva(tipo(clone(escreva)))
"#,
        "8\nnumero\n9\nverdadeiro falso\nfuncao\n",
    );
}
#[test]
fn capture_analysis_handles_control_flow_and_loop_locals() {
    check(
        r#"
f = funcao(n) {
    soma = 0
    para i em 0..=n { se i == 3 { continue } soma += i se i == 4 { pare } }
    se n > 0 { local = 7 soma += local } senao { local = 2 soma += local }
    soma
}
escreva(clone(f)(8))
g = funcao(n) { x = 0 enquanto x < n { x += 1 } (1 / 0) se_falhar x }
escreva(clone(g)(3))
"#,
        "14\n3\n",
    );
}
#[test]
fn captured_bindings_recursion_and_dynamic_environment_are_rejected() {
    error(
        "fator = 2\nf = funcao(x) { x * fator }\nclone([f])",
        "fator",
    );
    error(
        "f = funcao(n) { se n == 0 { 0 } senao { f(n - 1) } }\nclone(f)",
        "`f`",
    );
    error(
        "f = funcao() { avalie(\"1 + 2\") }\nclone(f)",
        "dinamicamente",
    );
    error("f = funcao() { catalogo() }\nclone(f)", "dinamicamente");
    error(
        "outer = funcao(x) { funcao() { x } }\nclone(outer(2))",
        "`x`",
    );
    error(
        "f = funcao(callback) { callback(\"segredo\") }\nclone(f)",
        "chamada indireta",
    );
    error(
        "f = funcao(xs, callback) { ordene(xs, callback) }\nclone(f)",
        "ordene",
    );
}
#[test]
fn modules_cannot_be_cloned_even_inside_a_collection() {
    error("m = importe \"matriz\"\nclone([m])", "módulo");
}
#[test]
#[cfg(feature = "graphics")]
fn native_resources_cannot_be_enabled_by_editing_the_visible_marker() {
    check(
        r#"
g = importe "tela_grafica"
c = importe "cartesiano"
t = g::nova(8, 8)
escreva(t:_clonavel)
t:_clonavel = verdadeiro
escreva(clone(t) se_falhar "recurso")
u = remova(t, :_clonavel)
escreva(clone([u]) se_falhar "aninhado")
d = c::documento()
escreva(clone(d) se_falhar "documento")
"#,
        "falso\nrecurso\naninhado\ndocumento\n",
    );
}
#[test]
#[cfg(feature = "graphics")]
fn image_snapshots_remain_independent_after_canvas_changes() {
    check(
        r#"
g = importe "tela_grafica"
t = g::nova(2, 2)
g::cor(t, :vermelho)
g::retangulo(t, 0, 0, 1, 1)
a = g::como_imagem(t)
b = clone(a)
g::apague(t)
g::cor(t, :azul)
g::retangulo(t, 0, 0, 1, 1)
c = g::como_imagem(t)
escreva(a == b, a == c)
"#,
        "verdadeiro falso\n",
    );
}
#[test]
fn clone_cancellation_unwinds_and_does_not_run_following_code() {
    use expressa::runtime::{DebugAction, DebugCtx, DebugHook, LeiaHost, run_with_hook};
    use std::{cell::Cell, rc::Rc};
    struct Hook(Rc<Cell<bool>>);
    impl DebugHook for Hook {
        fn before_stmt(&mut self, ctx: &DebugCtx<'_>) -> DebugAction {
            if ctx.span.line == 3 {
                self.0.set(true);
            }
            DebugAction::Continue
        }
        fn interrupted(&mut self) -> bool {
            self.0.get()
        }
    }
    struct Input;
    impl LeiaHost for Input {
        fn ask(&mut self, _: &str) -> Result<String, String> {
            unreachable!()
        }
    }
    let armed = Rc::new(Cell::new(false));
    let mut out = Vec::new();
    let code = run_with_hook(
        "a = [0]\nrepita 3000 vezes { a = [a] }\nclone(a)\nescreva(99)",
        "cancel.lep",
        Box::new(Hook(Rc::clone(&armed))),
        None,
        None,
        &mut out,
        &mut Vec::new(),
        &mut Input,
        &[],
    )
    .unwrap();
    assert!(armed.get());
    assert_eq!(code, 0);
    assert!(out.is_empty());
}

#[test]
fn images_have_shared_object_identity_and_independent_clone_buffers() {
    use expressa::runtime::{
        DebugAction, DebugCtx, DebugHook, Imagem, LEIA_MARKER, Value, run_with_hook,
    };
    use std::{cell::Cell, rc::Rc, sync::Arc};
    struct Hook {
        done: Rc<Cell<bool>>,
    }
    impl DebugHook for Hook {
        fn before_stmt(&mut self, ctx: &DebugCtx<'_>) -> DebugAction {
            if ctx.span.line == 1 {
                ctx.env.borrow_mut().define(
                    "a",
                    Value::imagem(Imagem::nova(1, 1, vec![1, 2, 3, 255]).unwrap()),
                );
            }
            if ctx.span.line == 3 {
                let env = ctx.env.borrow();
                let Value::Imagem(a) = env.vars.get("a").unwrap() else {
                    panic!()
                };
                let Value::Imagem(b) = env.vars.get("b").unwrap() else {
                    panic!()
                };
                let Value::Imagem(c) = env.vars.get("c").unwrap() else {
                    panic!()
                };
                assert!(Rc::ptr_eq(a, b));
                assert!(!Rc::ptr_eq(a, c));
                assert!(Arc::ptr_eq(&a.borrow().px, &c.borrow().px));
                Arc::make_mut(&mut a.borrow_mut().px)[0] = 9;
                assert_eq!(b.borrow().px[0], 9);
                assert_eq!(c.borrow().px[0], 1);
                self.done.set(true);
            }
            DebugAction::Continue
        }
    }
    struct Input;
    impl expressa::runtime::LeiaHost for Input {
        fn ask(&mut self, _prompt: &str) -> Result<String, String> {
            Err(LEIA_MARKER.into())
        }
    }
    let done = Rc::new(Cell::new(false));
    run_with_hook(
        "b = a\nc = clone(a)\n0",
        "image.lep",
        Box::new(Hook {
            done: Rc::clone(&done),
        }),
        None,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Input,
        &[],
    )
    .unwrap();
    assert!(done.get());
}

#[test]
#[cfg(all(unix, feature = "process"))]
fn a_running_process_cannot_be_cloned() {
    error(
        "p = importe \"processo\"\nx = p::linhas_de(\"printf x\")\nclone([x])",
        "processo",
    );
}

#[test]
fn clone_work_budget_rejects_an_oversized_matrix() {
    let err = run_with_options(
        "m = importe \"matriz\"\na = m::zeros(1024)\nclone(a)",
        "budget.lep",
        "",
        options(ExecutionBackend::Bytecode),
    )
    .unwrap_err();
    assert!(err.message.contains("orçamento"), "{err}");
    assert_eq!(err.span.line, 3);
}
