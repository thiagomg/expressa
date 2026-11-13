# Migração para base zero

Esta é uma mudança incompatível: programas antigos precisam ser revisados.
Não há detecção automática nem modo misto de indexação.

| Operação | Antes | Agora |
|---|---|---|
| Primeiro item ou caractere | `xs[1]` | `xs[0]` |
| Último item | `xs[tamanho(xs)]` | `xs[tamanho(xs) - 1]` |
| Primeira célula de matriz | `m[1, 1]` | `m[0, 0]` |
| Primeiros três itens | `xs[1..3]` | `xs[0..3]` (teto exclusivo) |
| Remover o primeiro item | `xs.remova(1)` | `xs.remova(0)` |
| Primeiro argumento do programa | `argumentos[1]` | `argumentos[0]` |
| Primeiro valor em formato manual | `"{1}"` | `"{0}"` |
| Canto superior esquerdo do terminal | `bloco(1, 1)` | `bloco(0, 0)` |
| Busca sem resultado | `procurar(xs, valor) == 0` | `procure(xs, valor) == -1` |

Para testar se encontrou, use `indice >= 0`. O índice `0` é um resultado válido.
Textos continuam indexados por caracteres Unicode, e não por bytes.
Índices negativos são erros; `-1` não acessa o último item.

Fatias e `remova` usam faixa com **teto exclusivo**. `[0..1]` contém o
primeiro item, `[1..]` começa no segundo e vai até o fim, e limites acima do
tamanho são recortados. Fatias invertidas ou que começam após o último item
são vazias. Um acesso simples fora dos limites continua sendo erro.
`remova` sem teto (`xs.remova(1..)`) é erro: o teto precisa aparecer.

Laços de contagem usam a mesma faixa: `para i em 0..tamanho(xs)` (exclusivo)
ou `para i em 1..=n` (inclusivo no teto escrito). O `para i de … até …` saiu
da linguagem.

```text
para i em 0..tamanho(xs) {
    escreva(xs[i])
}
```

Com uma lista vazia, esse laço não executa. Quando o índice não for necessário,
prefira `para item em xs`.

As telas gráficas já usavam origem zero. A ordem dos argumentos permanece:
matrizes e blocos usam `(linha, coluna)`; desenho gráfico usa `(x, y)`.
O plano cartesiano mantém suas coordenadas matemáticas.

Tamanhos, quantidades, datas, códigos de saída e chaves numéricas de mapas
não são índices e não mudam. Linhas e colunas de diagnóstico, depuração e
navegação no editor continuam começando em 1.
