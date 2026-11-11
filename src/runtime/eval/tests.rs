use super::*;
use crate::runtime::debug::DebugAction;

fn run(src: &str) -> String {
    run_to_string(src, "teste.lep").unwrap_or_else(|e| panic!("run failed: {e}"))
}

fn run_in(src: &str, input: &str) -> String {
    let mut input = io::Cursor::new(input.to_string());
    let mut out = Vec::new();
    let mut err = Vec::new();
    run_with(
        src,
        "teste.lep",
        Box::new(NoopHook),
        &mut input,
        &mut out,
        &mut err,
        None,
        None,
        None,
        &[],
    )
    .unwrap_or_else(|e| panic!("run failed: {e}"));
    String::from_utf8_lossy(&out).into_owned()
}

fn run_args(src: &str, args: &[&str], input: &str) -> String {
    let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    let mut input = io::Cursor::new(input.to_string());
    let mut out = Vec::new();
    let mut err = Vec::new();
    run_with(
        src,
        "teste.lep",
        Box::new(NoopHook),
        &mut input,
        &mut out,
        &mut err,
        None,
        None,
        None,
        &args,
    )
    .unwrap_or_else(|e| panic!("run failed: {e}"));
    String::from_utf8_lossy(&out).into_owned()
}

fn run_err(src: &str) -> String {
    match run_to_string(src, "teste.lep") {
        Err(e) => e.message,
        Ok(out) => panic!("expected error, got output {out:?}"),
    }
}

#[test]
fn media_example() {
    let src = include_str!("../../../exemplos/linguagem/media.lep");
    let out = run_to_string(src, "exemplos/linguagem/media.lep").unwrap();
    assert_eq!(out, "Média: 7,3\nResultado: Aprovado\n");
}

#[test]
fn example_programs_run() {
    let examples = [
        (
            "exemplos/linguagem/tabuada.lep",
            include_str!("../../../exemplos/linguagem/tabuada.lep"),
        ),
        (
            "exemplos/linguagem/fatorial.lep",
            include_str!("../../../exemplos/linguagem/fatorial.lep"),
        ),
        (
            "exemplos/linguagem/filtra.lep",
            include_str!("../../../exemplos/linguagem/filtra.lep"),
        ),
        (
            "exemplos/linguagem/closures.lep",
            include_str!("../../../exemplos/linguagem/closures.lep"),
        ),
        (
            "exemplos/linguagem/usa_matematica.lep",
            include_str!("../../../exemplos/linguagem/usa_matematica.lep"),
        ),
        (
            "exemplos/texto/poema.lep",
            include_str!("../../../exemplos/texto/poema.lep"),
        ),
        (
            "exemplos/texto/palindromo.lep",
            include_str!("../../../exemplos/texto/palindromo.lep"),
        ),
        (
            "exemplos/texto/ascii.lep",
            include_str!("../../../exemplos/texto/ascii.lep"),
        ),
        (
            "exemplos/programas/receita.lep",
            include_str!("../../../exemplos/programas/receita.lep"),
        ),
        (
            "exemplos/programas/turma.lep",
            include_str!("../../../exemplos/programas/turma.lep"),
        ),
        (
            "exemplos/programas/caixa.lep",
            include_str!("../../../exemplos/programas/caixa.lep"),
        ),
        (
            "exemplos/programas/diario.lep",
            include_str!("../../../exemplos/programas/diario.lep"),
        ),
        (
            "exemplos/programas/contatos.lep",
            include_str!("../../../exemplos/programas/contatos.lep"),
        ),
        (
            "exemplos/escola/bhaskara.lep",
            include_str!("../../../exemplos/escola/bhaskara.lep"),
        ),
        (
            "exemplos/escola/juros.lep",
            include_str!("../../../exemplos/escola/juros.lep"),
        ),
        (
            "exemplos/escola/regra_de_tres.lep",
            include_str!("../../../exemplos/escola/regra_de_tres.lep"),
        ),
        (
            "exemplos/escola/progressoes.lep",
            include_str!("../../../exemplos/escola/progressoes.lep"),
        ),
        (
            "exemplos/escola/estatistica.lep",
            include_str!("../../../exemplos/escola/estatistica.lep"),
        ),
        (
            "exemplos/escola/pitagoras.lep",
            include_str!("../../../exemplos/escola/pitagoras.lep"),
        ),
        (
            "exemplos/escola/geometria.lep",
            include_str!("../../../exemplos/escola/geometria.lep"),
        ),
        (
            "exemplos/escola/mdc_mmc.lep",
            include_str!("../../../exemplos/escola/mdc_mmc.lep"),
        ),
        (
            "exemplos/escola/media_ponderada.lep",
            include_str!("../../../exemplos/escola/media_ponderada.lep"),
        ),
        (
            "exemplos/escola/cinematica.lep",
            include_str!("../../../exemplos/escola/cinematica.lep"),
        ),
        (
            "exemplos/escola/seno_cosseno.lep",
            include_str!("../../../exemplos/escola/seno_cosseno.lep"),
        ),
        (
            "exemplos/escola/calc_fracoes.lep",
            include_str!("../../../exemplos/escola/calc_fracoes.lep"),
        ),
    ];
    for (file, src) in examples {
        run_to_string(src, file).unwrap_or_else(|e| panic!("{file} failed: {e}"));
    }
}

#[test]
fn calc_fracoes_prints_with_como_texto() {
    let src = include_str!("../../../exemplos/escola/calc_fracoes.lep");
    let out = run_to_string(src, "exemplos/escola/calc_fracoes.lep").unwrap();
    assert!(out.contains("1/4 + 1/4 = 1/2"), "{out}");
}

#[test]
fn tartaruga_anda_para_cima_e_pinta() {
    let src = r#"
tartaruga = importe "../lib/tartaruga"
tela_grafica = importe "tela_grafica"
t = tartaruga::nova_tamanho(20, 20)
t.tartaruga::cor(:azul)
t.tartaruga::ande(5)
g = t:tela
escreva(g.tela_grafica::pixel(10, 10))
escreva(g.tela_grafica::pixel(10, 5))
escreva(g.tela_grafica::pixel(11, 10))
escreva(t:x)
escreva(t:y)
"#;
    let out = run_to_string(src, "exemplos/jogos/t.lep").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out, "azul\nazul\nbranco\n10\n5\n");
}

#[test]
fn tartaruga_subir_nao_pinta_e_olhe_direita() {
    let src = r#"
tartaruga = importe "../lib/tartaruga"
tela_grafica = importe "tela_grafica"
t = tartaruga::nova_tamanho(20, 20)
g = t:tela
t.tartaruga::subir()
t.tartaruga::ande(5)
escreva(g.tela_grafica::pixel(10, 10))
escreva(g.tela_grafica::pixel(10, 5))
t.tartaruga::casa()
t.tartaruga::baixar()
t.tartaruga::cor(:vermelho)
t.tartaruga::olhe(90)
t.tartaruga::ande(5)
escreva(g.tela_grafica::pixel(15, 10))
escreva(g.tela_grafica::pixel(10, 9))
"#;
    let out = run_to_string(src, "exemplos/jogos/t.lep").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out, "branco\nbranco\nvermelho\nbranco\n");
}

#[test]
fn tartaruga_catalogo_esconde_imports_internos() {
    let src = r#"
ache = funcao(lista, nome)
inicio
para item em lista
inicio
    se item:nome == nome
    inicio
        retorne verdadeiro
    fim
fim
falso
fim
tartaruga = importe "../lib/tartaruga"
membros = catalogo(tartaruga)
escreva(ache(membros, "ande"))
escreva(ache(membros, "velocidade"))
escreva(ache(membros, "_mat"))
escreva(ache(membros, "_ande"))
"#;
    let out = run_to_string(src, "exemplos/jogos/t.lep").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out, "verdadeiro\nverdadeiro\nfalso\nfalso\n");
}

#[test]
fn tartaruga_devagar_termina_no_mesmo_ponto() {
    let src = r#"
tartaruga = importe "../lib/tartaruga"
tela_grafica = importe "tela_grafica"
t = tartaruga::nova_tamanho(20, 20)
t.tartaruga::cor(:azul)
t.tartaruga::velocidade(0.001)
t:passo = 2
t.tartaruga::ande(6)
g = t:tela
escreva(ultimo(g:ops):tipo)
n = tamanho(g:ops)
escreva(n)
g:ops = remova(g:ops, n - 1, n - 1)
escreva(tamanho(g:ops))
escreva(g.tela_grafica::pixel(10, 10))
escreva(g.tela_grafica::pixel(10, 4))
escreva(g.tela_grafica::pixel(11, 10))
escreva(t:y)
"#;
    let out = run_to_string(src, "exemplos/jogos/t.lep").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(out, "imagem\n4\n3\nazul\nazul\nbranco\n4\n");
}

#[test]
fn erros_example_does_not_crash() {
    let src = include_str!("../../../exemplos/linguagem/erros.lep");
    let out = run_to_string(src, "exemplos/linguagem/erros.lep").unwrap();
    assert_eq!(out, "");
}

#[test]
fn sair_stops_the_program() {
    let src = r#"
escreva("antes")
saia(1)
escreva("depois")
"#;
    let mut input = io::Cursor::new("");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with(
        src,
        "t.lep",
        Box::new(NoopHook),
        &mut input,
        &mut out,
        &mut err,
        None,
        None,
        None,
        &[],
    )
    .unwrap();
    assert_eq!(code, 1);
    assert_eq!(String::from_utf8(out).unwrap(), "antes\n");
    assert_eq!(run(r#"saia()"#), "");
    let skipped = run(r#"
saia(0)
escreva("depois")
"#);
    assert_eq!(skipped, "");
    let not_caught = run(r#"
saia(1) se_falhar 0
escreva("depois")
"#);
    assert_eq!(not_caught, "");
}

#[test]
fn para_em_iterates_text() {
    assert_eq!(
        run(r#"
para ch em "olá"
inicio
escreva(ch)
fim
"#),
        "o\nl\ná\n"
    );
    assert_eq!(run(r#"para ch em "" { escreva(ch) }"#), "");
    let msg = run_err(
        r#"
para x em 10
inicio
escreva(x)
fim
"#,
    );
    assert!(msg.contains("lista, texto ou conjunto"), "{msg}");
}

#[test]
fn pare_and_continue_in_loops() {
    assert_eq!(
        run(r#"
para i de 1 ate 5
inicio
se i == 3 { pare }
escreva(i)
fim
"#),
        "1\n2\n"
    );
    assert_eq!(
        run(r#"
para i de 1 ate 5
inicio
se i == 3 { continue }
escreva(i)
fim
"#),
        "1\n2\n4\n5\n"
    );
    assert_eq!(
        run(r#"
para ch em "abcd"
inicio
se ch == "c" { pare }
escreva(ch)
fim
"#),
        "a\nb\n"
    );
    assert_eq!(
        run(r#"
i = 0
enquanto verdadeiro
inicio
i = i + 1
se i == 2 { continue }
escreva(i)
se i >= 3 { pare }
fim
"#),
        "1\n3\n"
    );
    let msg = run_err("pare");
    assert!(msg.contains("laço"), "{msg}");
    let msg = run_err("continue");
    assert!(msg.contains("laço"), "{msg}");
}

#[test]
fn retorne_leaves_the_function() {
    let src = r#"
busca = funcao(xs, alvo)
inicio
para x em xs
inicio
    se x == alvo
    inicio
        retorne verdadeiro
    fim
fim
falso
fim
escreva(busca([1, 2, 3], 2))
escreva(busca([1, 2, 3], 9))
"#;
    assert_eq!(run(src), "verdadeiro\nfalso\n");
    assert_eq!(
        run(r#"
f = funcao()
inicio
retorne
escreva("nao")
fim
escreva(f())
"#),
        "nada\n"
    );
    assert_eq!(
        run(r#"
soma = funcao(x, y)
inicio
x + y
fim
escreva(soma(2, 3))
"#),
        "5\n"
    );
    let msg = run_err("retorne 1");
    assert!(msg.contains("função"), "{msg}");
    let skipped = run(r#"
f = funcao()
inicio
se verdadeiro
inicio
    retorne 1
fim
senao
inicio
    0
fim
se_falhar 9
fim
escreva(f())
"#);
    assert_eq!(skipped, "1\n");
}

#[test]
fn shebang_is_ignored_at_runtime() {
    assert_eq!(run("#!/usr/bin/env expressa\nescreva(2 + 2)\n"), "4\n");
}

#[test]
fn conjunto_unique_contem_union_remova() {
    assert_eq!(
        run(r#"
s = conjunto([:ana, :bia, :ana, 1])
escreva(s contem :ana)
escreva(s contem :carlos)
escreva(tamanho(s))
s += :carlos
escreva(s contem :carlos)
s += conjunto([:bia, :dani])
escreva(tamanho(s))
s = s.remova(:ana)
escreva(s contem :ana)
para x em conjunto([10, 20])
inicio
escreva(x)
fim
"#),
        "verdadeiro\nfalso\n3\nverdadeiro\n5\nfalso\n10\n20\n"
    );
    assert_eq!(
        run("escreva(conjunto([1, 2]) == conjunto([2, 1]))"),
        "verdadeiro\n"
    );
    assert!(run_err("conjunto([[1]])").contains("inválido"));
}

#[test]
fn compound_assign() {
    assert_eq!(
        run(r#"
i = 10
i += 3
i -= 1
escreva(i)
nome = "Ana"
nome += " Silva"
escreva(nome)
xs = [1, 2]
xs += [3]
escreva(xs)
xs[0] += 10
escreva(xs)
"#),
        "12\nAna Silva\n[1, 2, 3]\n[11, 2, 3]\n"
    );
    assert!(
        run_err(
            r#"xs = [1]
xs -= [1]"#
        )
        .contains("numero")
    );
    assert_eq!(
        run(r#"
tentativas = []
para i de 1 ate 6 {
tentativas += [["", "", "", "", ""]]
}
escreva(tamanho(tentativas))
"#),
        "6\n"
    );
}

#[test]
fn arithmetic_and_escreva() {
    assert_eq!(run(r#"escreva(10 + 5 * 2)"#), "20\n");
    assert_eq!(run(r#"escreva(10 / 2)"#), "5\n");
    assert_eq!(run(r#"escreva(10 % 3)"#), "1\n");
}

#[test]
fn leia_linha_e_prompt() {
    assert_eq!(run_in(r#"escreva(leia())"#, "Ana\n"), "Ana\n");
    assert_eq!(
        run_in(
            r#"
nome = leia("Nome: ")
escreva("Oi " + nome)
"#,
            "Ana\n"
        ),
        "Nome: Oi Ana\n"
    );
    assert_eq!(
        run_in(r#"escreva(leia() se_falhar "vazio")"#, ""),
        "vazio\n"
    );
    assert_eq!(run_in(r#"escreva(leia())"#, "  x  \n"), "  x  \n");
}

#[test]
fn se_falhar_div_zero() {
    assert_eq!(run(r#"escreva(10 / 0 se_falhar 0)"#), "0\n");
}

#[test]
fn uncaught_div_zero() {
    let msg = run_err("10 / 0");
    assert!(msg.contains("divisão por zero"), "{msg}");
}

#[test]
fn function_cannot_assign_outer() {
    let src = r#"
x = 1
f = funcao()
inicio
x = 2
fim
f()
"#;
    let msg = run_err(src);
    assert!(msg.contains("função não pode alterar"), "{msg}");
    let msg = run_err(
        r#"
s = "ABC"
f = funcao()
inicio
s[0] = "x"
fim
f()
"#,
    );
    assert!(msg.contains("função não pode alterar"), "{msg}");
}

#[test]
fn block_can_assign_outer() {
    let src = r#"
x = 1
inicio
x = 2
fim
escreva(x)
"#;
    assert_eq!(run(src), "2\n");
}

#[test]
fn brace_blocks_run() {
    assert_eq!(run("f = funcao(n) { n * 2 }\nescreva(f(21))"), "42\n");
    assert_eq!(
        run(r#"escreva(se 2 > 1 { "sim" } senao { "nao" })"#),
        "sim\n"
    );
    assert_eq!(run("m = mapa([\"a\" -> 1])\nescreva(m[\"a\"])"), "1\n");
    assert_eq!(
        run(r#"
p = mapa([:nome -> "Thiago", :idade -> 25])
escreva(p:nome)
escreva(p[:nome])
escreva(:nome)
p = p.remova(:idade)
escreva(p contem :idade)
"#),
        "Thiago\nThiago\nnome\nfalso\n"
    );
    assert_eq!(
        run(r#"
opções = mapa([:nome -> "Thiago"])
args_pos = ["a"]
res = mapa([
:op -> opções,
:args -> args_pos,
])
escreva(res:args[0])
"#),
        "a\n"
    );
}

#[test]
fn matriz_ops() {
    let src = r#"
importe "matriz"
A = matriz([
[1, 2],
[3, 4],
])
escreva(A[0, 1])
escreva(tamanho(A))
escreva(tamanho(A[0]))
escreva(nlinhas(A))
escreva(ncolunas(A))
escreva(det(A))
B = transposta(A)
escreva(B[0, 1])
I = identidade(2)
C = A * I
escreva(C[1, 1])
"#;
    assert_eq!(run(src), "2\n2\n2\n2\n2\n-2\n3\n4\n");
}

#[test]
fn function_call_and_closure_read() {
    let src = r#"
x = 10
soma = funcao(a, b)
inicio
a + b + x
fim
escreva(soma(1, 2))
"#;
    assert_eq!(run(src), "13\n");
}

#[test]
fn named_inner_function_can_be_returned() {
    let src = r#"
faz_somador = funcao(n) {
soma = funcao(x) { x + n }
soma
}
mais3 = faz_somador(3)
escreva(mais3(10))
"#;
    assert_eq!(run(src), "13\n");
}

#[test]
fn inner_function_sees_reassigned_local() {
    let src = r#"
f = funcao() {
x = 1
g = funcao() { x }
x = 2
g()
}
escreva(f())
"#;
    assert_eq!(run(src), "2\n");
}

#[test]
fn loop_closure_keeps_last_i() {
    let src = r#"
f = 0
para i de 1 ate 3 {
f = funcao() { i }
}
escreva(f())
"#;
    assert_eq!(run(src), "3\n");
}

#[test]
fn para_range_and_para_in() {
    let src = r#"
s = 0
para i de 1 ate 3
inicio
s = s + i
fim
escreva(s)
t = 0
para n em [10, 20]
inicio
t = t + n
fim
escreva(t)
"#;
    assert_eq!(run(src), "6\n30\n");
}

#[test]
fn list_index_is_zero_based() {
    let src = r#"
a = [10, 20, 30, 40]
escreva(a[0])
escreva(a[1..2])
a[0] = 99
escreva(a[0])
"#;
    assert_eq!(run(src), "10\n[20, 30]\n99\n");
}

#[test]
fn zero_based_boundaries_and_map_keys() {
    assert_eq!(
        run("a = [10, 20]\na[1] = 99\nescreva(a[0], a[1])"),
        "10 99\n"
    );
    assert_eq!(run(r#"escreva("ação"[0], "ação"[3])"#), "a o\n");
    assert_eq!(
        run(r#"m = mapa([0 -> "zero", 1 -> "um"])
escreva(m[0], m[1])"#),
        "zero um\n"
    );
    for source in [
        "[10][-1]",
        "[10][1]",
        "[][0]",
        r#"""[0]"#,
        "[10][9223372036854775807]",
        "[10][0.5]",
    ] {
        assert!(!run_err(source).is_empty(), "{source}");
    }
}

#[test]
fn zero_based_slices_and_removal_keep_inclusive_ends() {
    assert_eq!(
        run(r#"
xs = [10, 20, 30]
escreva(xs[0..0], xs[1..2], xs[3..], xs[2..0], [][0..])
escreva(xs[0..9223372036854775807])
escreva(xs.remova(0), xs.remova(2), xs.remova(0, 2))
escreva("ação"[0..1], "ação".remova(0), "ação".remova(3))
"#),
        "[10] [20, 30] [] [] []\n[10, 20, 30]\n[20, 30] [10, 20] []\naç ção açã\n"
    );
    for source in ["[10][-1..0]", "[10][0..-1]", "[10].remova(-1)"] {
        assert!(run_err(source).contains(">= 0"), "{source}");
    }
}

#[test]
fn migrated_argument_parser_handles_first_last_and_empty() {
    let lib = include_str!("../../../exemplos/unix/lib/linha_cmd.lep");
    let source = format!(
        r#"{lib}
r = separa_args(["primeiro", "--nome", "Ana", "-v", "ultimo"], ["nome"])
escreva(r:pos[0], r:pos[1], r:op:nome, r:op:v)
escreva(separa_args([], []):pos)
"#
    );
    assert_eq!(run(&source), "primeiro ultimo Ana \n[]\n");
}

#[test]
fn migrated_games_accept_first_and_last_letters() {
    let termo = include_str!("../../../exemplos/jogos/termo.lep")
        .split("// Para trocar a palavra secreta")
        .next()
        .unwrap();
    assert_eq!(
        run(&format!(
            r#"{termo}
escreva(remove_letra("ABC", 0), remove_letra("ABC", 2))
escreva(compara_palavra("ABC", "ABC")[0]:valor)
escreva(compara_palavra("ABC", "CAB")[0]:valor)
escreva(como_lista(""))
"#
        )),
        "BC AB\nverde\namarelo\n[]\n"
    );
    let forca = include_str!("../../../exemplos/jogos/forca.lep")
        .split("// Troque a lista ou passe a palavra")
        .next()
        .unwrap();
    assert_eq!(
        run(&format!(
            r#"{forca}
escreva(e_letra("A"), e_letra("Z"), e_letra("?"))
escreva(ganhou("AZ", conjunto(["A", "Z"])))
escreva(tamanho(figura(0)) > 0, tamanho(figura(6)) > 0)
"#
        )),
        "verdadeiro verdadeiro falso\nverdadeiro\nverdadeiro verdadeiro\n"
    );
}

#[test]
fn como_texto_uses_map_hook() {
    let src = r#"
como_texto = funcao(fr) { formate("{}/{}", fr:numerador, fr:denominador) }
fr = mapa([:numerador -> 1, :denominador -> 4])
fr:_como_texto = como_texto
escreva(formate("{}", fr))
escreva(fr)
escreva("x" + fr)
escreva(fr.como_texto())
"#;
    assert_eq!(run(src), "1/4\n1/4\nx1/4\n1/4\n");
}

#[test]
fn como_texto_without_hook_keeps_mapa() {
    let out = run(r#"escreva(mapa([:a -> 1]))"#);
    assert!(out.contains("mapa"), "{out}");
    assert!(out.contains("a"), "{out}");
}

#[test]
fn como_texto_recursion_is_an_error() {
    let err = run_to_string(
        "fr = mapa([:n -> 1])\nfr:_como_texto = funcao(x) { formate(\"{}\", x) }\nescreva(fr)\n",
        "t.lep",
    )
    .unwrap_err();
    assert!(err.message.contains("mesmo mapa"), "{}", err.message);
}

#[test]
fn como_texto_must_return_text() {
    let err = run_to_string(
        "fr = mapa([])\nfr:_como_texto = funcao(x) { 1 }\nescreva(fr)\n",
        "t.lep",
    )
    .unwrap_err();
    assert!(err.message.contains("devolver texto"), "{}", err.message);
}

#[test]
fn map_colon_and_ufcs() {
    let src = r#"
p = mapa(["nome" -> "Ana"])
escreva(p:nome)
p:nome = "Bia"
escreva(p:nome)
escreva([10, 20, 30].tamanho())
"#;
    assert_eq!(run(src), "Ana\nBia\n3\n");
}

#[test]
fn maps_and_contem() {
    let src = r#"
p = mapa([
"nome" -> "Ana",
"idade" -> 25,
])
escreva(p["nome"])
p["cidade"] = "Fortaleza"
escreva(tamanho(p))
escreva(p contem "idade")
"#;
    assert_eq!(run(src), "Ana\n3\nverdadeiro\n");
}

#[test]
fn string_builtins() {
    let src = r#"
escreva(maiuscula("oi"))
escreva(junte(["a", "b"], "-"))
escreva(limpe("  x  "))
"#;
    assert_eq!(run(src), "OI\na-b\nx\n");
}

#[test]
fn plus_concatenates_when_either_side_is_text() {
    assert_eq!(run(r#"escreva("Média: " + 7.5)"#), "Média: 7,5\n");
    assert_eq!(run(r#"escreva(10 + " itens")"#), "10 itens\n");
    assert_eq!(run(r#"escreva("a" + "b")"#), "ab\n");
    assert_eq!(run(r#"escreva([1] + [2, 3])"#), "[1, 2, 3]\n");
}

#[test]
fn unary_and_logic_short_circuit() {
    assert_eq!(run(r#"escreva(-8)"#), "-8\n");
    assert_eq!(run(r#"escreva(nao falso)"#), "verdadeiro\n");
    assert_eq!(run(r#"escreva(falso e (1 / 0))"#), "falso\n");
    assert_eq!(run(r#"escreva(verdadeiro ou (1 / 0))"#), "verdadeiro\n");
}

#[test]
fn if_is_an_expression() {
    let src = r#"
r = se 1 > 2
inicio
"a"
fim
ou se 3 > 2
inicio
"b"
fim
senao
inicio
"c"
fim
escreva(r)
"#;
    assert_eq!(run(src), "b\n");
}

#[test]
fn block_value_is_the_last_expression() {
    let src = r#"
r = inicio
x = 1
x + 4
fim
escreva(r)
"#;
    assert_eq!(run(src), "5\n");
}

#[test]
fn block_locals_do_not_leak() {
    let src = r#"
inicio
y = 1
fim
escreva(y)
"#;
    let msg = run_err(src);
    assert!(msg.contains("não definida"), "{msg}");
}

#[test]
fn repita_runs_the_body_n_times() {
    assert_eq!(
        run(r#"
repita 3 vezes
inicio
escreva("x")
fim
"#),
        "x\nx\nx\n"
    );
}

#[test]
fn text_index_and_slice_are_zero_based() {
    assert_eq!(run(r#"escreva("abc"[0])"#), "a\n");
    assert_eq!(run(r#"escreva("abc"[1..2])"#), "bc\n");
    assert_eq!(run(r#"escreva("b"[0..2])"#), "b\n");
    assert_eq!(run(r#"escreva("bacia"[1..])"#), "acia\n");
    assert_eq!(run(r#"escreva("abc"[4..])"#), "\n");
    assert_eq!(run(r#"escreva([10, 20][0..8])"#), "[10, 20]\n");
    assert!(run_err(r#"escreva("b"[1])"#).contains("fora do intervalo"));
}

#[test]
fn text_index_assign_replaces_one_char() {
    assert_eq!(
        run(r#"
s = "ABCDE"
s[1] = "b"
escreva(s)
"#),
        "AbCDE\n"
    );
    assert_eq!(
        run(r#"
s = "ação"
s[1] = "Ç"
escreva(s)
"#),
        "aÇão\n"
    );
    assert_eq!(
        run(r#"
linhas = ["ABC", "DEF"]
linhas[0][1] = "x"
escreva(linhas)
"#),
        "[\"AxC\", \"DEF\"]\n"
    );
    assert_eq!(
        run(r#"
m = mapa([:s -> "ABC"])
m:s[1] = "x"
escreva(m:s)
"#),
        "AxC\n"
    );
    assert_eq!(
        run(r#"
s = "ABC"
a = s
s[0] = "x"
escreva(a)
escreva(s)
"#),
        "ABC\nxBC\n"
    );
    assert!(
        run_err(
            r#"
s = "ABC"
s[1] = "ab"
"#
        )
        .contains("tamanho 1")
    );
    assert!(
        run_err(
            r#"
s = "ABC"
s[1] = ""
"#
        )
        .contains("tamanho 1")
    );
    assert!(
        run_err(
            r#"
s = "ABC"
s[1] = 1
"#
        )
        .contains("caractere")
    );
    assert!(run_err(r#""ABC"[1] = "b""#).contains("literal"));
    assert!(
        run_err(
            r#"
s = "ABC"
s[-1] = "x"
"#
        )
        .contains(">= 0")
    );
    assert!(
        run_err(
            r#"
s = "ABC"
s[3] = "x"
"#
        )
        .contains("fora do intervalo")
    );
}

#[test]
fn se_falhar_catches_undefined_and_bad_index() {
    assert_eq!(run(r#"escreva(lista[0] se_falhar "não")"#), "não\n");
    assert_eq!(run(r#"escreva([1][99] se_falhar 0)"#), "0\n");
}

#[test]
fn function_arity_and_duplicate_params() {
    let arity = run_err(
        r#"
f = funcao(a, b)
inicio
a
fim
f(1)
"#,
    );
    assert!(arity.contains("espera 2 argumento"), "{arity}");

    let dup = run_err(
        r#"
f = funcao(a, a)
inicio
a
fim
"#,
    );
    assert!(dup.contains("parâmetro duplicado"), "{dup}");
}

#[test]
fn importe_with_and_without_alias() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("expressa-imp-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("aritmetica.lep"),
        "soma = funcao(a, b)\ninicio\n    a + b\nfim\n",
    )
    .unwrap();
    let main = dir.join("main.lep");
    let src = r#"
m = importe "aritmetica"
escreva(m::soma(2, 3))
importe "aritmetica"
escreva(soma(4, 5))
"#;
    let out = run_to_string(src, main.to_str().unwrap()).unwrap();
    assert_eq!(out, "5\n9\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn importe_achatado_nao_cobre_nome_existente() {
    let err = run_err(
        r#"
importe "tela"
importe "tela_grafica"
"#,
    );
    assert!(err.contains("já existe"), "{err}");
    assert!(err.contains("fundo"), "{err}");
    assert!(err.contains("m = importe \"tela_grafica\""), "{err}");
    assert!(run_err("importe \"tela\"\nimporte \"tela\"\n").contains("já existe"));
    assert_eq!(
        run(r#"
t = importe "tela"
g = importe "tela_grafica"
escreva(tipo(t::fundo), tipo(g::fundo))
"#),
        "funcao funcao\n"
    );
}

#[test]
fn par_mapa_conjunto_and_native_modules() {
    assert_eq!(
        run(r#"
p = :nome -> "Ana"
escreva(p:chave)
escreva(p:valor)
m = mapa([p, :idade -> 25])
escreva(m:nome)
escreva(tamanho(m))
s = conjunto([1, 1, 2])
escreva(tamanho(s))
escreva(tamanho(mapa()))
escreva(tamanho(conjunto()))
"#),
        "nome\nAna\nAna\n2\n2\n0\n0\n"
    );
    assert!(run_err("1 -> 2 -> 3").contains("encadeados"));
    assert!(run_err("mapa([:a -> 1, :a -> 2])").contains("duplicada"));
    assert!(run_err("[] -> 1").contains("chave de par"));
    assert_eq!(
        run(r#"
importe "matriz"
A = matriz([[1, 2], [3, 4]])
escreva(A.transposta()[0, 1])
m = importe "matriz"
escreva(A.m::transposta()[0, 1])
escreva(m::nlinhas(A))
"#),
        "3\n3\n2\n"
    );
    assert_eq!(
        run(r#"
matriz = importe "matriz"
A = matriz::matriz([[1, 2], [3, 4]])
escreva(A.matriz::transposta()[0, 1])
"#),
        "3\n"
    );
    assert!(run_err("zeros(2)").contains("não definida"));
}

#[test]
fn native_import_conflicts_with_file() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("expressa-nat-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("matriz.lep"), "x = 1\n").unwrap();
    let main = dir.join("main.lep");
    let err = match run_to_string(r#"importe "matriz""#, main.to_str().unwrap()) {
        Err(e) => e.message,
        Ok(out) => panic!("expected conflict, got {out:?}"),
    };
    assert!(err.contains("módulo nativo"), "{err}");
    assert!(err.contains("./matriz"), "{err}");
    let out = run_to_string(
        r#"
importe "./matriz"
escreva(x)
"#,
        main.to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(out, "1\n");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Runs on a plain test thread (2 MB of stack): the evaluator must grow
/// its own stack instead of relying on a big one.
fn run_fundo(src: &str) -> Result<String, RuntimeError> {
    run_to_string(src, "teste.lep")
}

#[test]
fn recursao_sem_fim_e_erro_e_nao_estouro_de_pilha() {
    let err = run_fundo("f = funcao(n) {\n    f(n + 1)\n}\nf(1)\n").unwrap_err();
    assert!(
        err.message.contains("recursão profunda demais"),
        "{}",
        err.message
    );
    assert_eq!(err.stack.len(), PROFUNDIDADE_MAXIMA);
    // Ten thousand identical frames print as one line plus a count.
    let text = err.to_string();
    assert!(text.lines().count() < 10, "{text}");
    assert!(text.contains("a mesma chamada, mais 9998 vezes"), "{text}");
}

#[test]
fn recursao_profunda_mas_finita_funciona() {
    let out = run_fundo(
        "f = funcao(n) {\n    se n == 0 { 0 } senao { 1 + f(n - 1) }\n}\nescreva(f(9000))\n",
    )
    .unwrap();
    assert_eq!(out, "9.000\n");
}

#[test]
fn se_falhar_pega_recursao_sem_fim() {
    let out =
        run_fundo("f = funcao(n) {\n    f(n + 1)\n}\nescreva(f(1) se_falhar \"parou\")\n").unwrap();
    assert_eq!(out, "parou\n");
}

#[test]
fn avaliar_recursivo_tambem_tem_limite() {
    let err = run_fundo("f = funcao() {\n    avalie(\"f()\")\n}\nf()\n").unwrap_err();
    assert!(
        err.message.contains("recursão profunda demais"),
        "{}",
        err.message
    );
}

// ── Valores que contêm a si mesmos e estruturas muito fundas ─────

#[test]
fn lista_e_mapa_que_contem_a_si_mesmos_imprimem_e_comparam() {
    let out = run("a = mapa([:x -> 1])\n\
         a[\"eu\"] = a\n\
         escreva(a)\n\
         xs = [1, 2]\n\
         xs[0] = xs\n\
         escreva(xs)\n\
         escreva(a == a, xs == xs, xs contem xs)\n\
         b = mapa([:x -> 1])\n\
         b[\"eu\"] = b\n\
         escreva(a == b)\n\
         b[\"x\"] = 2\n\
         escreva(a == b)\n\
         escreva(\"lista: \" + xs)\n\
         escreva(formate(\"{}\", [xs]))\n");
    assert_eq!(
        out,
        "mapa{\"x\": 1, \"eu\": mapa{…}}\n\
         [[…], 2]\n\
         verdadeiro verdadeiro verdadeiro\n\
         verdadeiro\n\
         falso\n\
         lista: [[…], 2]\n\
         [[[…], 2]]\n"
    );
}

#[test]
fn mesma_lista_duas_vezes_nao_e_ciclo() {
    let out = run("a = [1]\nb = [a, a]\nescreva(b)\n");
    assert_eq!(out, "[[1], [1]]\n");
}

#[test]
fn estrutura_muito_funda_imprime_compara_e_e_liberada() {
    // Built by the program, not by the parser: no depth limit applies.
    let out = run("x = []\n\
         y = []\n\
         repita 100000 vezes {\n\
         \x20   x = [x]\n\
         \x20   y = [y]\n\
         }\n\
         escreva(x == y)\n\
         escreva(tamanho(\"\" + x))\n");
    assert_eq!(out, "verdadeiro\n200.002\n");
}

// ── Programas no limite de profundidade do parser ───────────────

/// The largest `n` whose program `gera(n)` still parses.
fn maior_aceito(gera: impl Fn(usize) -> String) -> usize {
    let (mut ok, mut falha) = (0, 1);
    while crate::parser::parse(&gera(falha)).is_ok() {
        ok = falha;
        falha *= 2;
    }
    while falha - ok > 1 {
        let meio = (ok + falha) / 2;
        if crate::parser::parse(&gera(meio)).is_ok() {
            ok = meio;
        } else {
            falha = meio;
        }
    }
    ok
}

#[test]
fn todo_programa_aceito_pelo_parser_roda_sem_estourar_a_pilha() {
    // Runs on a plain test thread (2 MB, debug build): the checks after
    // parsing, evaluation, and dropping the tree must all fit.
    type Gera = fn(usize) -> String;
    let formas: [(&str, Gera); 11] = [
        ("parênteses", |n| {
            format!("x = {}1{}\nescreva(x)\n", "(".repeat(n), ")".repeat(n))
        }),
        ("soma", |n| {
            format!("x = 1{}\nescreva(x)\n", " + 1".repeat(n))
        }),
        ("e", |n| {
            format!("x = verdadeiro{}\nescreva(x)\n", " e verdadeiro".repeat(n))
        }),
        ("nao", |n| {
            format!("x = {}verdadeiro\nescreva(x)\n", "nao ".repeat(n))
        }),
        ("menos", |n| format!("x = {}1\nescreva(x)\n", "-".repeat(n))),
        ("listas", |n| {
            format!(
                "x = {}{}\nescreva(tamanho(x))\n",
                "[".repeat(n),
                "]".repeat(n)
            )
        }),
        ("blocos", |n| {
            format!("{}escreva(1)\n{}", "{\n".repeat(n), "}\n".repeat(n))
        }),
        ("chamadas", |n| {
            format!("x = {}1{}\n", "tamanho(".repeat(n), ")".repeat(n))
        }),
        ("índices", |n| {
            format!("x = [1]\ny = x{}\n", "[1]".repeat(n))
        }),
        ("se", |n| {
            format!(
                "x = {}1{}\nescreva(x)\n",
                "se verdadeiro { ".repeat(n),
                " } senao { 0 }".repeat(n)
            )
        }),
        ("funções", |n| {
            format!(
                "f = {}1{}\nescreva(f())\n",
                "funcao() { ".repeat(n),
                " }".repeat(n)
            )
        }),
    ];
    for (nome, gera) in formas {
        let n = maior_aceito(gera);
        assert!(n >= 100, "{nome}: só {n} níveis");
        // Any result is fine (some shapes fail at run time on purpose);
        // the point is that the process is still here.
        let _ = run_to_string(&gera(n), "teste.lep");
        let erro = crate::parser::parse(&gera(n + 1)).unwrap_err();
        assert!(
            erro.message.contains("aninhada demais"),
            "{nome}: {}",
            erro.message
        );
    }
}

// ── Funções locais e memória (ajustes/01-modelo-de-memoria.md) ─────

/// Max live frames seen before any statement, and live frames left
/// after the run.
fn frames_vivos(src: &str) -> (usize, usize) {
    use crate::runtime::env::ENVS_VIVOS;
    use std::cell::Cell;

    struct Contador(Rc<Cell<usize>>);
    impl DebugHook for Contador {
        fn before_stmt(&mut self, _ctx: &DebugCtx<'_>) -> DebugAction {
            let vivos = ENVS_VIVOS.with(Cell::get);
            self.0.set(self.0.get().max(vivos));
            DebugAction::Continue
        }
    }

    let antes = ENVS_VIVOS.with(Cell::get);
    let max = Rc::new(Cell::new(0));
    let mut input = io::Cursor::new(String::new());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    run_with(
        src,
        "teste.lep",
        Box::new(Contador(Rc::clone(&max))),
        &mut input,
        &mut out,
        &mut err,
        None,
        None,
        None,
        &[],
    )
    .unwrap_or_else(|e| panic!("run failed: {e}"));
    let depois = ENVS_VIVOS.with(Cell::get);
    (max.get() - antes, depois - antes)
}

/// `exemplos/jogos/conway_jogo_da_vida.lep` without its endless main
/// loop, running `geracoes` generations of the glider pattern.
fn conway(geracoes: usize) -> String {
    let src = include_str!("../../../exemplos/jogos/conway_jogo_da_vida.lep");
    let funcoes = src.split("// Rotina principal do jogo").next().unwrap();
    format!(
        "{funcoes}\nm = zeros(10, 25)\nm.predefinido()\n\
         repita {geracoes} vezes {{\n    m = m.nova_geracao()\n}}\n"
    )
}

#[test]
fn funcoes_locais_que_nao_escapam_nao_acumulam_frames() {
    // `vizinhos` defines `pos_lin`/`pos_col`, which close over its frame.
    // Each call must free that frame, or every generation strands one
    // frame per cell (and the old matrix each one holds).
    let (max_2, sobra_2) = frames_vivos(&conway(2));
    let (max_6, sobra_6) = frames_vivos(&conway(6));
    assert_eq!(max_2, max_6, "frames crescem com o número de gerações");
    assert_eq!(
        (sobra_2, sobra_6),
        (0, 0),
        "frames vivos depois da execução"
    );
}

fn devolve_recursiva(chamadas: usize) -> String {
    format!(
        "faz = funcao(k) {{\n\
         \x20   g = funcao(n) {{ se n == 0 {{ k }} senao {{ g(n - 1) }} }}\n\
         \x20   retorne g\n\
         }}\n\
         s = 0\n\
         para i de 1 ate {chamadas} {{ s += faz(i)(2) }}\n\
         escreva(s)\n"
    )
}

#[test]
fn funcao_que_escapa_e_depois_e_descartada_e_liberada_durante_a_execucao() {
    // Each `faz` frame escapes with `g` and so survives its own end;
    // once `g` is dropped the periodic check frees it.
    let (max_poucas, _) = frames_vivos(&devolve_recursiva(3_000));
    let (max_muitas, sobra) = frames_vivos(&devolve_recursiva(9_000));
    assert!(
        max_muitas <= max_poucas + 16,
        "frames crescem com as chamadas: {max_poucas} → {max_muitas}"
    );
    assert_eq!(sobra, 0);
}

#[test]
fn verificacao_periodica_nao_esvazia_frame_em_execucao() {
    // `cria()` registers thousands of frames while `usa` still runs, so
    // the periodic check happens with `usa`'s frame (and `g`) live.
    let out = run("cria = funcao() { funcao() { 0 } }\n\
         usa = funcao() {\n\
         \x20   g = funcao(n) { se n == 0 { 7 } senao { g(n - 1) } }\n\
         \x20   repita 5000 vezes { cria() }\n\
         \x20   g(3)\n\
         }\n\
         escreva(usa())\n");
    assert_eq!(out, "7\n");
}

#[test]
fn funcao_local_que_escapa_por_mapa_continua_funcionando() {
    let out = run("guardadas = mapa([:f -> 0])\n\
         registra = funcao(m) {\n\
         \x20   g = funcao(n) { se n == 0 { 0 } senao { g(n - 1) } }\n\
         \x20   m[\"f\"] = g\n\
         \x20   0\n\
         }\n\
         registra(guardadas)\n\
         escreva(guardadas[\"f\"](3))\n");
    assert_eq!(out, "0\n");
}

#[test]
fn funcao_local_que_escapa_por_lista_continua_funcionando() {
    let out = run("guardadas = [0]\n\
         registra = funcao(xs) {\n\
         \x20   g = funcao(n) { se n == 0 { 0 } senao { g(n - 1) } }\n\
         \x20   xs[0] = g\n\
         \x20   0\n\
         }\n\
         registra(guardadas)\n\
         escreva(guardadas[0](3))\n");
    assert_eq!(out, "0\n");
}

#[test]
fn funcao_local_recursiva_devolvida_funciona() {
    let out = run("faz = funcao() {\n\
         \x20   g = funcao(n) { se n == 0 { 0 } senao { g(n - 1) } }\n\
         \x20   retorne g\n\
         }\n\
         escreva(faz()(3))\n");
    assert_eq!(out, "0\n");
}

#[test]
fn funcao_local_recursiva_devolvida_numa_lista_funciona() {
    let out = run("faz = funcao() {\n\
         \x20   g = funcao(n) { se n == 0 { 0 } senao { g(n - 1) } }\n\
         \x20   retorne [g]\n\
         }\n\
         escreva(faz()[0](3))\n");
    assert_eq!(out, "0\n");
}

#[test]
fn call_stack_names_the_callee() {
    let err = match run_to_string("boom = funcao() {\n    1 / 0\n}\nboom()\n", "app.lep") {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert_eq!(err.stack.len(), 2, "{}", err);
    assert_eq!(err.stack[0].name, "<modulo>");
    assert_eq!(err.stack[0].span.line, 4);
    assert_eq!(err.stack[1].name, "boom");
    assert_eq!(err.stack[1].span.line, 2);
    let text = err.to_string();
    assert!(text.contains("0: boom (app.lep:2:"), "{text}");
    assert!(text.contains("1: <modulo> (app.lep:4:"), "{text}");
}

#[test]
fn call_stack_nested_functions_use_each_frame_line() {
    let err = match run_to_string(
        "foo = funcao() {\n    bar()\n}\nbar = funcao() {\n    1 / 0\n}\nfoo()\n",
        "app.lep",
    ) {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert_eq!(
        err.stack
            .iter()
            .map(|f| (f.name.as_str(), f.span.line))
            .collect::<Vec<_>>(),
        [("<modulo>", 7), ("foo", 2), ("bar", 5)]
    );
}

#[test]
fn top_level_error_omits_the_module_frame() {
    let err = match run_to_string("1 / 0\n", "app.lep") {
        Err(e) => e,
        Ok(_) => panic!("expected error"),
    };
    assert_eq!(err.stack.len(), 1);
    assert!(!err.shows_stack());
    assert!(!err.to_string().contains("pilha de chamadas"), "{err}");
}

#[test]
fn helpers_used_by_eval() {
    assert_eq!(
        base_dir_of("exemplos/linguagem/media.lep").as_os_str(),
        "exemplos/linguagem"
    );
    assert_eq!(base_dir_of("media.lep").as_os_str(), ".");
    assert_eq!(
        Value::Numero(3.0.try_into().unwrap()).format_with(NumeroLocale::PtBr),
        "3"
    );
    assert_eq!(
        Value::Texto("a".into()).format_with(NumeroLocale::PtBr),
        "a"
    );
    assert!(
        check_unique_params(&[Param {
            name: "a".into(),
            span: Span::default(),
        }])
        .is_ok()
    );
    assert_eq!(
        check_unique_params(&[
            Param {
                name: "a".into(),
                span: Span::default(),
            },
            Param {
                name: "a".into(),
                span: Span::default(),
            },
        ])
        .unwrap_err(),
        "a"
    );
}

#[test]
fn time_limit_stops_a_long_loop() {
    let src = r#"
repita 1000000 vezes
inicio
x = 1
fim
"#;
    let err = run_to_string_with(src, "t.lep", "", None, Some(Duration::from_millis(30)))
        .expect_err("should time out");
    assert!(err.message.contains("tempo esgotado"), "{err}");
}

#[test]
fn workspace_root_blocks_path_escape() {
    let dir = std::env::temp_dir().join("expressa-ws-test");
    std::fs::create_dir_all(&dir).unwrap();
    let src = r#"
importe "arquivo"
leia_arquivo("../secret.txt")
"#;
    let file = dir.join("main.lep");
    let err = run_to_string_with(src, file.to_str().unwrap(), "", Some(dir.clone()), None)
        .expect_err("should block escape");
    assert!(err.message.contains("fora da pasta"), "{err}");
}

/// Runs `src` under a ChannelDebugger, answering every pause with
/// `continuar`; returns the pauses seen and the result.
fn debug_pauses(
    src: &str,
) -> (
    Vec<super::super::debug::DebugPaused>,
    Result<i32, RuntimeError>,
) {
    use super::super::debug::ChannelDebugger;
    let (pause_tx, pause_rx) = std::sync::mpsc::channel();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<String>();
    let src = src.to_string();
    let runner = std::thread::spawn(move || {
        let mut input = io::Cursor::new("");
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_with(
            &src,
            "t.lep",
            Box::new(ChannelDebugger::new(pause_tx, cmd_rx)),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            &[],
        )
    });
    let mut pauses = Vec::new();
    while let Ok(p) = pause_rx.recv() {
        pauses.push(p);
        let _ = cmd_tx.send("continuar".into());
    }
    (pauses, runner.join().unwrap())
}

/// Starts `src` under a ChannelDebugger on a thread: (pauses, commands,
/// finished) channels.
fn debug_session(
    src: &str,
) -> (
    std::sync::mpsc::Receiver<super::super::debug::DebugPaused>,
    std::sync::mpsc::Sender<String>,
    std::sync::mpsc::Receiver<Result<i32, RuntimeError>>,
) {
    use super::super::debug::ChannelDebugger;
    let (pause_tx, pause_rx) = std::sync::mpsc::channel();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<String>();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let src = src.to_string();
    std::thread::spawn(move || {
        let mut input = io::Cursor::new("");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let r = run_with(
            &src,
            "t.lep",
            Box::new(ChannelDebugger::new(pause_tx, cmd_rx)),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            &[],
        );
        let _ = done_tx.send(r);
    });
    (pause_rx, cmd_tx, done_rx)
}

const WAIT: std::time::Duration = std::time::Duration::from_secs(5);

#[test]
fn debugger_stops_when_asked_while_running() {
    let (pauses, cmds, done) = debug_session("enquanto verdadeiro {\n    x = 1\n}\n");
    pauses.recv_timeout(WAIT).expect("pause on start");
    cmds.send("continuar".into()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    cmds.send("terminar".into()).unwrap();
    assert!(
        done.recv_timeout(WAIT).is_ok(),
        "Parar after Continuar must end the program"
    );
}

#[test]
fn breakpoint_added_while_running_is_hit() {
    let (pauses, cmds, done) = debug_session("enquanto verdadeiro {\n    x = 1\n    y = 2\n}\n");
    pauses.recv_timeout(WAIT).expect("pause on start");
    cmds.send("continuar".into()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    cmds.send("ponto t.lep:3".into()).unwrap();
    let p = pauses
        .recv_timeout(WAIT)
        .expect("stops at the new breakpoint");
    assert_eq!(p.line, 3);
    // Removed while running: Continuar runs freely again until Parar.
    cmds.send("remover t.lep:3".into()).unwrap();
    // While paused, a breakpoint change answers with a fresh snapshot.
    assert_eq!(pauses.recv_timeout(WAIT).unwrap().line, 3);
    cmds.send("continuar".into()).unwrap();
    assert!(
        pauses
            .recv_timeout(std::time::Duration::from_millis(300))
            .is_err(),
        "breakpoint was removed"
    );
    cmds.send("terminar".into()).unwrap();
    assert!(done.recv_timeout(WAIT).is_ok());
}

#[test]
fn parar_interrupts_a_long_durma() {
    let started = std::time::Instant::now();
    let (pauses, cmds, done) = debug_session("durma(30)\n");
    pauses.recv_timeout(WAIT).expect("pause on start");
    cmds.send("continuar".into()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    cmds.send("terminar".into()).unwrap();
    assert!(done.recv_timeout(WAIT).is_ok());
    assert!(started.elapsed() < std::time::Duration::from_secs(5));

    // Same with Executar's hook.
    use super::super::debug::StopHook;
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut input = io::Cursor::new("");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let r = run_with(
            "durma(30)\n",
            "t.lep",
            Box::new(StopHook::new(rx)),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            &[],
        );
        let _ = done_tx.send(r);
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    tx.send("terminar".into()).unwrap();
    assert!(
        done_rx.recv_timeout(WAIT).is_ok(),
        "Executar: Parar during durma"
    );
}

#[test]
fn debugger_pauses_on_runtime_error_with_locals() {
    let (pauses, result) = debug_pauses("f = funcao(x) {\n    y = x * 2\n    y / 0\n}\nf(3)\n");
    assert!(result.is_err());
    // First the pause on start, then the error.
    assert_eq!(pauses.len(), 2, "{pauses:?}");
    let p = &pauses[1];
    assert!(p.error.as_deref().unwrap_or("").contains("zero"), "{p:?}");
    assert_eq!(p.line, 3);
    let var = |n: &str| p.vars.iter().find(|v| v.name == n).map(|v| v.value.clone());
    assert_eq!(var("x").as_deref(), Some("3"));
    assert_eq!(var("y").as_deref(), Some("6"));
    assert_eq!(p.stack[0].name, "f", "innermost frame first: {:?}", p.stack);
}

#[test]
fn errors_and_pauses_in_imported_functions_name_their_file() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("expressa-mod-file-{nanos}"));
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::write(
        dir.join("lib/m.lep"),
        "divide = funcao(a, b) {\n    q = a / b\n    q\n}\n",
    )
    .unwrap();
    let main = dir.join("main.lep");
    let src = "importe \"lib/m\"\nescreva(divide(1, 0))\n";
    let err = run_to_string(src, main.to_str().unwrap()).unwrap_err();
    assert!(err.file.ends_with("lib/m.lep"), "{err}");
    assert_eq!(err.span.line, 2);
    // Innermost frame is the imported function; the caller is below.
    assert!(
        err.stack.last().unwrap().file.ends_with("lib/m.lep"),
        "{:?}",
        err.stack
    );
    assert!(err.stack[0].file.ends_with("main.lep"), "{:?}", err.stack);

    // The debugger stops in the module, showing the module's line.
    use super::super::debug::ChannelDebugger;
    let (pause_tx, pause_rx) = std::sync::mpsc::channel();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<String>();
    let main_s = main.to_str().unwrap().to_string();
    let runner = std::thread::spawn(move || {
        let mut input = io::Cursor::new("");
        let (mut out, mut errb) = (Vec::new(), Vec::new());
        run_with(
            src,
            &main_s,
            Box::new(ChannelDebugger::new(pause_tx, cmd_rx)),
            &mut input,
            &mut out,
            &mut errb,
            None,
            None,
            None,
            &[],
        )
    });
    let mut last = None;
    while let Ok(p) = pause_rx.recv() {
        let _ = cmd_tx.send("continuar".into());
        last = Some(p);
    }
    let _ = runner.join();
    let p = last.unwrap();
    assert!(p.error.is_some());
    assert!(p.file.ends_with("lib/m.lep"), "{p:?}");
    assert_eq!(p.source_line.trim(), "q = a / b");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn debugger_ignores_errors_handled_by_se_falhar() {
    let (pauses, result) = debug_pauses("x = 1 / 0 se_falhar 7\nescreva(x)\n");
    assert_eq!(result.unwrap(), 0);
    assert!(pauses.iter().all(|p| p.error.is_none()), "{pauses:?}");
}

#[test]
fn leia_host_receives_prompt() {
    struct Capture {
        prompt: String,
        reply: String,
    }
    impl LeiaHost for Capture {
        fn ask(&mut self, prompt: &str) -> Result<String, String> {
            self.prompt = prompt.to_string();
            Ok(self.reply.clone())
        }
    }
    let mut host = Capture {
        prompt: String::new(),
        reply: "Ana".into(),
    };
    let mut input = io::Cursor::new("");
    let mut out = Vec::new();
    let mut err = Vec::new();
    run_with(
        r#"escreva(leia("Seu nome:"))"#,
        "t.lep",
        Box::new(NoopHook),
        &mut input,
        &mut out,
        &mut err,
        None,
        None,
        Some(&mut host),
        &[],
    )
    .unwrap();
    assert_eq!(host.prompt, "Seu nome:");
    assert_eq!(String::from_utf8(out).unwrap(), "Ana\n");
}

#[test]
fn debugger_hook_sees_statements() {
    use std::cell::Cell;

    struct Shared(Rc<Cell<usize>>);
    impl DebugHook for Shared {
        fn before_stmt(&mut self, _ctx: &DebugCtx<'_>) -> DebugAction {
            self.0.set(self.0.get() + 1);
            DebugAction::Continue
        }
    }

    let src = "x = 1\ny = 2\nescreva(x + y)\n";
    let n = Rc::new(Cell::new(0));
    let mut buf = Vec::new();
    let mut err = Vec::new();
    let mut input = io::Cursor::new("");
    run_with(
        src,
        "t.lep",
        Box::new(Shared(Rc::clone(&n))),
        &mut input,
        &mut buf,
        &mut err,
        None,
        None,
        None,
        &[],
    )
    .unwrap();
    assert_eq!(n.get(), 3);
    assert_eq!(String::from_utf8(buf).unwrap(), "3\n");
}

#[test]
fn enquanto_counts() {
    let src = r#"
i = 1
enquanto i <= 3
inicio
escreva(i)
i = i + 1
fim
"#;
    assert_eq!(run(src), "1\n2\n3\n");
    assert_eq!(
        run(r#"
i = 0
enquanto falso
inicio
escreva(i)
fim
escreva("ok")
"#),
        "ok\n"
    );
    let msg = run_err(
        r#"
enquanto 1
inicio
escreva(1)
fim
"#,
    );
    assert!(msg.contains("esperado bool"), "{msg}");
}

#[test]
fn leia_linhas_reads_until_eof() {
    assert_eq!(
        run_in(r#"escreva(leia_linhas())"#, "a\nb\nc\n"),
        "[\"a\", \"b\", \"c\"]\n"
    );
    assert_eq!(run_in(r#"escreva(leia_linhas())"#, ""), "[]\n");
    assert_eq!(
        run_in(
            r#"
a = leia()
escreva(a)
escreva(leia_linhas())
"#,
            "um\ndois\ntres\n"
        ),
        "um\n[\"dois\", \"tres\"]\n"
    );
    assert_eq!(
        run_in(r#"escreva(leia_linhas())"#, "sozinha"),
        "[\"sozinha\"]\n"
    );
}

#[test]
fn argumentos_are_zero_based() {
    assert_eq!(run(r#"escreva(argumentos)"#), "[]\n");
    assert_eq!(
        run_args(r#"escreva(argumentos[0])"#, &["Thiago"], ""),
        "Thiago\n"
    );
    assert_eq!(
        run_args(r#"escreva(tamanho(argumentos))"#, &["a", "b"], ""),
        "2\n"
    );
    let msg = run_err(r#"argumentos = 1"#);
    assert!(msg.contains("nome nativo"), "{msg}");
}

#[test]
fn grep_filter_with_args_and_leia_linhas() {
    let src = r#"
busca = argumentos[0]
para linha em leia_linhas()
inicio
se linha contem busca
inicio
    escreva(linha)
fim
fim
"#;
    assert_eq!(
        run_args(src, &["Thiago"], "Ana\nThiago Silva\nBruno\nThiago\n"),
        "Thiago Silva\nThiago\n"
    );
}

#[cfg(unix)]
mod process_lifetime {
    use super::*;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::sync::atomic::{AtomicU64, Ordering};

    const WAIT: Duration = Duration::from_secs(10);
    const CHILD: &str = "runtime::eval::tests::process_lifetime::probe_child";

    // Invoked explicitly by the lifetime tests in a separate test executable.
    #[test]
    #[ignore = "subprocess helper, invoked explicitly by the lifetime tests"]
    fn probe_child() {
        use std::io::Read;
        let path = std::env::var_os("EXPRESSA_TEST_PROBE").expect("probe socket");
        let mut stream = UnixStream::connect(path).unwrap();
        // The controller owns the deadline. A child timer could make a leaked
        // process look like a successful kill. If the controller disappears,
        // its socket closes and read_exact fails instead of leaving an orphan.
        stream.write_all(b"R").unwrap();
        let mut command = [0];
        stream.read_exact(&mut command).unwrap();
        assert_eq!(&command, b"G");
        stream.write_all(b"A").unwrap();
    }

    struct Probe {
        listener: UnixListener,
        stream: Option<UnixStream>,
        alive: bool,
        checked: Rc<std::cell::Cell<bool>>,
    }
    impl DebugHook for Probe {
        fn before_stmt(&mut self, ctx: &DebugCtx<'_>) -> DebugAction {
            use std::io::Read;
            let line = ctx
                .source
                .lines()
                .nth(ctx.span.line as usize - 1)
                .unwrap()
                .trim();
            if line == "retorne child" {
                assert!(self.stream.is_none(), "duplicate readiness checkpoint");
                let deadline = Instant::now() + WAIT;
                let (mut stream, _) = loop {
                    match self.listener.accept() {
                        Ok(pair) => break pair,
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "child never connected");
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(e) => panic!("probe accept: {e}"),
                    }
                };
                stream.set_read_timeout(Some(WAIT)).unwrap();
                stream.set_write_timeout(Some(WAIT)).unwrap();
                let mut ready = [0];
                stream.read_exact(&mut ready).unwrap();
                assert_eq!(&ready, b"R");
                self.stream = Some(stream);
            } else if line == "observe = 0" {
                let stream = self.stream.as_mut().expect("readiness handshake");
                let mut answer = [0];
                if self.alive {
                    stream.write_all(b"G").expect("retained child was killed");
                    stream
                        .read_exact(&mut answer)
                        .expect("retained child did not acknowledge");
                    assert_eq!(&answer, b"A");
                } else {
                    // A leaked child stays blocked awaiting G: timeout is failure.
                    assert_eq!(
                        stream.read(&mut answer).expect("child did not terminate"),
                        0
                    );
                }
                self.checked.set(true);
            }
            DebugAction::Continue
        }
    }

    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn check(body: &str, alive: bool, backend: ExecutionBackend) {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temp = Temp(std::env::temp_dir().join(format!(
            "exp-probe-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        std::fs::create_dir_all(&temp.0).unwrap();
        let path = temp.0.join("socket");
        let listener = UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        let command = format!(
            "proc::linhas_de([\"env\", {:?}, {:?}, \"--exact\", {:?}, \"--ignored\", \"--nocapture\"])",
            format!("EXPRESSA_TEST_PROBE={}", path.display()),
            std::env::current_exe().unwrap().to_str().unwrap(),
            CHILD,
        );
        let source = format!(
            "proc = importe \"processo\"\nmake = funcao() {{\nchild = {command}\nretorne child\n}}\n{}",
            body.replace("ARG", "make()")
        );
        let checked = Rc::new(std::cell::Cell::new(false));
        let hook = Probe {
            listener,
            stream: None,
            alive,
            checked: Rc::clone(&checked),
        };
        run_configured_with_io(
            &source,
            "lifetime.lep",
            Box::new(hook),
            &mut io::Cursor::new(""),
            &mut Vec::new(),
            &mut Vec::new(),
            ExecutionOptions {
                backend,
                ..ExecutionOptions::default()
            },
            None,
            None,
            false,
            None,
            &[],
        )
        .unwrap();
        assert!(checked.get(), "observation checkpoint not reached");
    }

    #[test]
    fn discarded_process_values_keep_the_ast_cleanup_timing() {
        for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode] {
            check("ARG\nobserve = 0", false, backend);
            check("repita 1 vezes {\nARG\nobserve = 0\n}", true, backend);
        }
    }

    #[test]
    fn stack_arguments_release_processes_on_return_and_arity_error() {
        for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode] {
            check("f = funcao(p) { 0 }\nf(ARG)\nobserve = 0", false, backend);
            check(
                "f = funcao() { 0 }\nf(ARG) se_falhar 0\nobserve = 0",
                false,
                backend,
            );
            check(
                "f = funcao(p) { funcao() { p } }\ng = f(ARG)\nobserve = 0",
                true,
                backend,
            );
        }
    }
}

#[test]
fn parameter_slots_are_visible_to_debugger_and_collector() {
    struct Hook(Rc<RefCell<Vec<String>>>);
    impl DebugHook for Hook {
        fn before_stmt(&mut self, ctx: &DebugCtx<'_>) -> DebugAction {
            if ctx
                .source
                .lines()
                .nth(ctx.span.line as usize - 1)
                .unwrap()
                .trim()
                == "observe = 0"
            {
                let bindings = super::super::debug::collect_vars(ctx);
                self.0.borrow_mut().extend(
                    bindings
                        .into_iter()
                        .filter(|b| b.name == "arg")
                        .map(|b| b.value),
                );
            }
            DebugAction::Continue
        }
    }
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode] {
        let values = Rc::new(RefCell::new(Vec::new()));
        let before = super::super::env::ENVS_VIVOS.with(|n| n.get());
        let source = "f = funcao(arg) {\navalie(\"arg = 7\")\nobserve = 0\ng = funcao() { arg }\narg = g\n0\n}\nf(1)";
        run_configured_with_io(
            source,
            "slots.lep",
            Box::new(Hook(Rc::clone(&values))),
            &mut io::Cursor::new(""),
            &mut Vec::new(),
            &mut Vec::new(),
            ExecutionOptions {
                backend,
                ..ExecutionOptions::default()
            },
            None,
            None,
            false,
            None,
            &[],
        )
        .unwrap();
        assert_eq!(&*values.borrow(), &["7"]);
        assert_eq!(
            super::super::env::ENVS_VIVOS.with(|n| n.get()),
            before,
            "{backend:?}: parameter cycle leaked"
        );
    }
}
