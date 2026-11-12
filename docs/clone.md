# Referências e `clone`

Listas, mapas, conjuntos, matrizes e imagens compartilham identidade de objeto na atribuição e passagem para funções. Reatribuir uma variável não reatribui as outras. Operações existentes que retornam novas coleções, como `remova`, continuam retornando novos valores; isso não é uma mutação implícita do argumento.

```text
a = [[1]]
b = a
c = clone(a)
b[0][0] = 9
escreva(a)  // [[9]]
escreva(c)  // [[1]]
```

`clone(valor)` está no núcleo, disponível também no perfil mínimo. Não é palavra-chave. O nome passa a ocupar o binding nativo `clone`; código que o usava como variável externa precisa escolher outro nome. As regras existentes de parâmetros e bindings nativos continuam valendo.

## O que é copiado

| Valor | Comportamento de `clone` |
| --- | --- |
| Lista, mapa, conjunto, matriz | Outro objeto, com dados mutáveis independentes |
| Par | Mesma chave por valor e valor interno clonado recursivamente |
| Texto, número, bool, nada | Valor equivalente |
| Imagem | Outro objeto imagem; pixels imutáveis inicialmente compartilhados |
| Função Expressa comprovadamente sem capturas | Outra closure com código compartilhado e ambiente separado |
| Função nativa | Mesma referência à função nativa, que não contém ambiente lexical |
| Módulo ou processo | Erro, inclusive em coleções |
| Mapa não clonável ou recurso nativo | Erro, inclusive em coleções |

A mesma origem mutável gera um único correspondente na cópia. Se duas posições compartilhavam uma lista, continuam compartilhando a mesma **nova** lista. Ciclos também são preservados. As closures clonadas não retêm os frames do programa original; seus nomes nativos permitidos são instalados num ambiente separado.

`clone` não executa callbacks de clonagem nem o `_como_texto` dos mapas. A operação falha sem alterar o original nem devolver cópia parcial. Objetos parciais são liberados mesmo se já contiverem ciclos. Diagnósticos incluem um caminho, como `$[1]:callback`; caminhos muito profundos são abreviados.

## Textos

Textos têm comportamento de valor. Escrever num caractere ou trecho substitui o texto no destino, respeitando suas permissões de escrita:

```text
a = "oi"
b = a
a[0] = "O"
escreva(b)  // oi
```

Em uma lista compartilhada, substituir um elemento texto é observado pelos aliases da **lista**. Isso não modifica o texto anterior. A escrita e a ordem de avaliação existentes não foram alteradas. Esta entrega não muda `String` para armazenamento compartilhado: compartilhar caracteres é uma otimização futura permitida pelo contrato, não um benefício de desempenho já implementado.

## Funções sem capturas

```text
dobro = funcao(x) { x * 2 }
outro = clone(dobro)
escreva(outro(3))  // 6
```

Parâmetros e nomes locais definitivamente disponíveis nos caminhos de execução são permitidos. Dependências externas contam como capturas mesmo quando contêm números ou textos imutáveis. Isso inclui atribuições que dependem da existência de um binding externo. Recursão por um nome externo é uma captura: reatribuir esse nome pode mudar o comportamento.

Nativas do núcleo podem ser referenciadas sem capturar o programa. Uma nativa importada diretamente pode ser clonada como valor; uma função que acessa um módulo ou alias externo continua dependendo daquele binding.

A classificação é **conservadora**. São recusadas funções que usam `avalie` ou `catalogo` e chamadas indiretas cuja ausência de acesso ao ambiente não é demonstrável. `ordene` também é recusada numa função clonada, pois seu callback pode ser uma nativa que avalia texto no ambiente do chamador. Essas restrições evitam aceitar código que alcance estado externo através de aliases ou callbacks. Uma função sem nomes livres pode, portanto, receber erro de classificação quando faz chamadas indiretas; isso não significa que ela foi comprovada como tendo uma captura.

```text
fator = 2
f = funcao(x) { x * fator }
clone(f)  // erro: depende de fator

aplicar = funcao(callback, x) { callback(x) }
clone(aplicar)  // erro: chamada indireta não comprovada
```

A análise é feita sobre instruções, tanto no AST (compilação temporária somente para análise) como no bytecode e EXB. O texto de depuração de um EXB não determina o resultado. Nomes de diagnóstico das chamadas também não são usados como prova da função executada. Não há análise adicional na criação de funções comuns; ela ocorre quando `clone` encontra uma função.

Funções aninhadas são analisadas junto com seus protótipos: podem depender de parâmetros/locais do novo ambiente de chamada, mas não de estado externo da closure original. Uma closure retornada que captura um parâmetro não se torna clonável individualmente só porque a função que a criou era clonável. A classificação considera os bindings existentes no momento de `clone`; a cópia não acompanha novas declarações ou reatribuições do ambiente original.

## Mapas não clonáveis e recursos

```text
a = mapa()
a:_clonavel = falso
clone(a)  // erro
```

Ausência do campo ou `verdadeiro` permite a clonagem de mapas comuns; `falso` impede. Outro tipo no campo é erro. O campo é um bool, não uma função, e pode ser alterado em mapas comuns.

Telas gráficas e documentos cartesianos recebem `:_clonavel = falso` ao serem criados. Também carregam uma marca interna: trocar/remover o campo não torna o recurso clonável. `remova` preserva essa política ao produzir outro mapa a partir do recurso. O marcador não é um sandbox nem rastreia reconstruções arbitrárias de dados pelo programa.

## Imagens e snapshots

No runtime, `a = imagem` compartilha o objeto; `clone(imagem)` cria outro objeto. Cada objeto contém um buffer de pixels compartilhável e imutável. Uma futura escrita pode separar o buffer com COW sem separar os aliases do mesmo objeto imagem.

`como_imagem(tela)` e as imagens guardadas nos comandos de desenho são snapshots: alterações posteriores na origem não alteram seus pixels. `rotacione` e `redimensione` continuam retornando novas imagens. Esta entrega não adiciona escrita de pixels nem funções de efeito como `avermelhe`.

Para hosts Rust, `Value::Imagem` agora contém `Rc<RefCell<Imagem>>`; use `Value::imagem(dados)` para construir um objeto novo. Copiar `Value` compartilha esse objeto. Para escrever pixels no host preservando snapshots, use `Arc::make_mut` no buffer do objeto, ou substitua seu buffer. Não altere buffers compartilhados por mecanismos que violem a imutabilidade de `Arc`.

## Limites de implementação e validação

A travessia de dados usa fila explícita e memoização. Há orçamento de 1.000.000 unidades de trabalho para visitas, arestas e células de matrizes. A análise de funções usa 200.000 unidades ponderadas de estados e profundidade máxima de 64 protótipos. São limites desta implementação, não um orçamento global de heap. Cópias de textos/chaves, metadados do alocador e outras operações do runtime ainda não têm um limite global de memória.

As reservas de saída de listas, mapas e matrizes são falíveis. A clonagem e a análise verificam prazo/cancelamento cooperativamente. A compilação temporária de um corpo AST usa o compilador existente e não é preemptível; o prazo é conferido pela análise depois dessa preparação. O collector continua responsável por cópias com ciclos que se tornem inacessíveis depois do retorno.

Testes em `tests/clone.rs` cobrem AST, bytecode e EXB, aliases, ciclos, marcadores, funções, recursos, textos, snapshots e identidade/COW de imagens. Testes internos verificam que falhas não acumulam ciclos parciais e que a análise ignora texto de depuração e nomes de chamada forjados.

[Contrato aprovado e acompanhamento](../lang-tasks/023-copy-and-alias-contract.md). [Medições antes/depois](../benchmarks/CLONE.md).
