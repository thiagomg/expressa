// Shared by EXB unit tests and runtime profile integration tests.
// Each program checks its result and uses only features in the minimal build.
pub const WORKLOADS: &[(&str, &str)] = &[
    ("startup.lep", "escreva(verdadeiro)"),
    (
        "arithmetic.lep",
        "soma = 0\npara n em 1..=100 { soma += n }\nescreva(soma == 5050)",
    ),
    (
        "avalie.lep",
        "x = 2\nr = avalie(\"x += 3\\nx * 2\")\nescreva(x == 5 e r == 10)",
    ),
    (
        "closures.lep",
        "criar = funcao(x) { funcao(y) { x + y } }\nf = criar(7)\nescreva(f(3) == 10)",
    ),
    (
        "collections.lep",
        "xs = [1, 2, 3]\nm = mapa([:dados -> xs])\nxs[1] = 9\nescreva(m:dados[1] == 9 e tamanho(xs) == 3)",
    ),
    (
        "cycles.lep",
        "xs = [nada]\nxs[0] = xs\nxs[0][0] = 7\nescreva(xs[0] == 7)",
    ),
    (
        "matrix.lep",
        "importe \"matriz\"\na = matriz([[1, 2], [3, 4]])\nb = a * a\nescreva(b[0, 0] == 7 e b[1, 1] == 22)",
    ),
    (
        "recursion.lep",
        "soma = funcao(n) { se n == 0 { 0 } senao { n + soma(n - 1) } }\nescreva(soma(100) == 5050)",
    ),
    (
        "arguments.lep",
        "zero = funcao() { 1 }\nsoma = funcao(p0, p1, p2, p3, p4, p5, p6, p7) { p0 + p1 + p2 + p3 + p4 + p5 + p6 + p7 }\nescreva(zero() + soma(1, 2, 3, 4, 5, 6, 7, 8) == 37)",
    ),
];
