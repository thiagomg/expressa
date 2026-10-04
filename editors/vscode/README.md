# Expressa para VS Code

Suporte à linguagem [Expressa](https://github.com/thiagomg/expressa) (arquivos `.lep`).

- **Cores** para palavras-chave (`se`, `para`, `repita`, `inicio`/`fim`…), funções nativas, textos, números, `:chaves`, `modulo::nome` e comentários.
- **Autocompletar**
  - funções nativas com a assinatura e a documentação de `docs/nativas/*.lep`;
  - funções de módulo (`pinte`, `zeros`, `leia_arquivo`…) acrescentam o `importe "tela"` que faltar;
  - nomes definidos no arquivo e nos `.lep` importados;
  - `m::` lista o módulo importado como `m = importe "…"`;
  - `x.` sugere funções no estilo `x.tamanho()`;
  - dentro de `importe "` lista os módulos nativos e os `.lep` da pasta.
- **Ajuda** ao passar o mouse e enquanto digita os argumentos (`pinte(texto, frente, fundo)`).
- **Ir para a definição** (F12) de funções e variáveis, inclusive em arquivos importados, e do próprio `importe "lib/arquivo"`.
- Trechos prontos: `se`, `se senao`, `para de`, `para em`, `repita`, `enquanto`, `funcao`.
- Enter depois de `{` ou `inicio` indenta; `}` e `fim` voltam.

## Gerar e instalar

Precisa de Node.js 16+.

```sh
cd editors/vscode
sh scripts/empacotar.sh              # gera expressa-0.1.0.vsix
code --install-extension expressa-0.1.0.vsix
```

`sh scripts/empacotar.sh --instalar` faz os dois. No VS Code também dá: *Extensões → … → Instalar do VSIX*.

## Manter em dia com a linguagem

As palavras-chave vêm de `src/lexer/tokens.rs`, as funções nativas de `src/runtime/nativas/mod.rs` e a documentação de `docs/nativas/*.lep`. Depois de mudar a linguagem:

```sh
node scripts/gerar.js           # atualiza src/nativas.json e a gramática
node scripts/gerar.js --check   # falha se estiverem desatualizados (para CI)
npm test
```

O empacotamento já roda `gerar.js` e os testes.
