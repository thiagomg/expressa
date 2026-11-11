# Arquivos Expressa Bytecode (.exb), versão 1

## Uso

```sh
cargo build -p expressa --release --bin expressa --offline --locked
target/release/expressa compilar programa.lep
target/release/expressa compilar programa.lep -o pasta/programa.exb
target/release/expressa programa.exb argumento1
target/release/expressa debug programa.exb
target/release/expressa --marcador-leia programa.exb
```

`compilar` não executa o programa. A saída padrão troca a extensão para `.exb`; `-o` também exige essa extensão. A escrita usa arquivo temporário no mesmo diretório e rename, preservando um artefato anterior se a compilação falhar. O fonte não pode ser usado como saída.

O arquivo contém instruções, tabelas e o texto-fonte necessário aos diagnósticos. Não é uma forma de ocultar o fonte nem um executável nativo. A execução usa o backend bytecode; `--backend ast` é rejeitado para `.exb`. `--ast` continua destinado ao `.lep`.

O loader não reinterpreta o texto-fonte embutido. Ele decodifica e valida instruções, reconstruindo os enums atuais da VM em memória. A compactação em memória descartada na etapa 05 não foi retomada: as instruções permanecem com o layout atual, de 16 bytes nesta plataforma. O formato de arquivo é independente desse layout.

## Módulos, caminhos e avaliação dinâmica

```text
m = importe "./biblioteca.exb"
escreva(m::soma(2, 3))
```

Módulos `.exb` são validados antes da execução e usam o mesmo cache, detecção de ciclos e preflight de bindings das importações existentes. Imports sem extensão continuam procurando `.lep`; não há reescrita nem empacotamento automático de dependências. Módulos nativos continuam disponíveis. Arquivos `.lep` também podem importar `.exb` ao usar o backend bytecode.

Os caminhos relativos de imports e operações de arquivo são resolvidos a partir do arquivo executado/importado, inclusive quando o `.exb` foi movido para outra pasta. O nome do fonte armazenado é usado para diagnósticos e pilhas, não para redirecionar o diretório de execução. As regras existentes para chamadas de funções exportadas continuam valendo.

O `.lep` original não precisa existir. Imports que ainda apontam para `.lep` continuam exigindo seus fontes e usando o parser. `avalie` continua compilando explicitamente o texto recebido em runtime; isso também exige o frontend. O arquivo não congela locale nem o modo decimal/float: literais são preservados exatamente, e as opções de execução continuam controlando as operações.

## API Rust

- `compile_exb(source, source_name) -> Result<Vec<u8>, ExbError>`: compila sem executar.
- `load_exb(bytes)` e `read_exb(path)`: retornam um `CompiledProgram` opaco, imutável e validado.
- `run_exb_with_options(bytes, execution_file, stdin, options)`: carrega e executa, capturando saída/erros/código de saída.
- `run_compiled_with_options(program, execution_file, stdin, options)`: reutiliza código já validado; cada execução cria novos globais.
- `run_exb_file_args(path, args, ExbRunMode)`: entrada CLI com modos `Run`, `Debug` e `Marker`.

`execution_file` define a base dos caminhos relativos. O texto e o nome originais ficam disponíveis por `CompiledProgram::source()`/`source_name()`. Código e tabelas internas não são mutáveis pela API. `ExbError` informa mensagem e offset; erros de execução continuam sendo `RuntimeError`, com os spans e frames do fonte original.

## Contrato binário v1

Todos os inteiros têm largura fixa e ordem **little-endian**. Textos são UTF-8 com comprimento em **bytes** u32; contagens e índices são u32, nunca `usize` ou ponteiros Rust. Nenhuma representação de `enum`, `Rc`, `HashMap` ou `Value` é gravada diretamente.

| Offset | Tamanho | Campo |
| --- | ---: | --- |
| 0 | 4 | Magic `45 58 42 00` (`EXB\0`) |
| 4 | 2 | Versão, u16 = 1 |
| 6 | 2 | Recursos, u16 = 3: decimal96 e fonte de depuração presente |
| 8 | 4 | Comprimento do payload u32 |
| 12 | 8 | FNV-1a-64 do payload, u64 |
| 20 | variável | Payload |

A checksum começa em `0xcbf29ce484222325`; para cada byte, aplica xor e multiplicação por `0x100000001b3`, módulo 2^64. Ela detecta corrupção, não autentica o autor. A validação estrutural é feita mesmo quando a checksum está correta. Versões, recursos desconhecidos, bytes excedentes e truncamento são rejeitados.

Payload: `string(source_name)`, `string(source_text)`, span do programa e chunk raiz. Um span é a sequência u32 `(linha, coluna, início_byte, fim_byte)`, com fim exclusivo. Linha/coluna começam em 1 e coluna conta caracteres, como o lexer. O span artificial `(0,0,0,0)` é permitido para `Nada`/`Finish`; instruções com efeito/diagnóstico precisam de uma posição real. Offsets devem ser limites UTF-8 válidos e corresponder ao texto embutido.

Chunk, na ordem:

1. Contagem e spans.
2. Contagem e nomes UTF-8.
3. Contagem e constantes: tag 0 + decimal de 16 bytes, ou tag 1 + string.
4. Contagem e protótipos de função: parâmetros (nome + span), doc opcional, span e chunk filho.
5. Contagem e imports: alias opcional, caminho, doc opcional e span.
6. Contagem e laços: tipo, variável opcional, spans inicial/final, saída e próximo passo.
7. Contagem e instruções: tag u8, operandos específicos e índice de span u32.

Uma string opcional usa byte 0/1 seguido da string quando presente. O tipo de laço é 0 repeat, 1 range, 2 items, 3 while; variável opcional usa byte 0/1 e índice u32. Destinos de controle usam **ordinais de instrução**, não offsets em bytes. O mapa nome→posição dos parâmetros é reconstruído da lista ordenada de parâmetros do protótipo; não é serializado como HashMap. Duplicatas permanecem sujeitas ao erro de definição de função já existente na linguagem, inclusive em código não executado.

Decimal: quatro u32 LE, na ordem flags, magnitude baixa, média e alta. A magnitude tem 96 bits. Nas flags, bit 31 é sinal e bits 16–23 são escala de 0 a 28; os demais bits precisam ser zero. Sinal, escala e precisão são preservados, incluindo zero com sinal. Não há conversão intermediária por f64 ou texto.

## Opcodes v1

A coluna abaixo fixa os números de arquivo. `(value)`/`target` são operandos u32; `Call` leva `argc` e `name`, ambos u32. Operações sem campos não têm payload. Booleanos, operações unárias/binárias e os dois sentidos de curto-circuito têm tags próprias. Toda instrução termina com o índice de span u32.

| Tag | Operação lógica |
| --- | --- |
| 0 | `Op::Nada` |
| 1 | `Op::Drop` |
| 2 | `Op::SetLast` |
| 3 | `Op::Pair` |
| 4 | `Op::Index` |
| 5 | `Op::Index2` |
| 6 | `Op::Return` |
| 7 | `Op::Finish` |
| 8 | `Op::Checkpoint` |
| 9 | `Op::EnterScope` |
| 10 | `Op::LeaveScope` |
| 11 | `Op::ParentScope` |
| 12 | `Op::RestoreScope` |
| 13 | `Op::CheckBool` |
| 14 | `Op::CheckInt` |
| 15 | `Op::EndTry` |
| 16 | `Op::LoopBody` |
| 17 | `Op::LoopEnd` |
| 18 | `Op::Break` |
| 19 | `Op::Continue` |
| 20 | `Op::ReceiverIndex` |
| 21 | `Op::ReceiverIndex2` |
| 22 | `Op::MatrixDestination` |
| 23 | `Op::ReadDestination` |
| 24 | `Op::WriteDestination` |
| 25 | `Op::Constant(value)` |
| 26 | `Op::BadNumber(value)` |
| 27 | `Op::Load(value)` |
| 28 | `Op::Store(value)` |
| 29 | `Op::LoadLocal(value)` |
| 30 | `Op::StoreLocal(value)` |
| 31 | `Op::List(value)` |
| 32 | `Op::Field(value)` |
| 33 | `Op::MapField(value)` |
| 34 | `Op::Closure(value)` |
| 35 | `Op::Jump(value)` |
| 36 | `Op::BranchFalse(value)` |
| 37 | `Op::Try(value)` |
| 38 | `Op::Import(value)` |
| 39 | `Op::LoopEnter(value)` |
| 40 | `Op::LoopNext(value)` |
| 41 | `Op::NameDestination(value)` |
| 42 | `Op::ReceiverName(value)` |
| 43 | `Op::ReceiverMap(value)` |
| 44 | `Op::MapDestination(value)` |
| 45 | `Op::Bool(false)` |
| 46 | `Op::Bool(true)` |
| 47 | `Op::Slice(false)` |
| 48 | `Op::Slice(true)` |
| 49 | `Op::ReceiverReadOnly(false)` |
| 50 | `Op::ReceiverReadOnly(true)` |
| 51 | `Op::IndexDestination(false)` |
| 52 | `Op::IndexDestination(true)` |
| 53 | `Op::Unary(UnaryOp::Neg)` |
| 54 | `Op::Unary(UnaryOp::Not)` |
| 55 | `Op::Binary(BinaryOp::Add)` |
| 56 | `Op::Binary(BinaryOp::Sub)` |
| 57 | `Op::Binary(BinaryOp::Mul)` |
| 58 | `Op::Binary(BinaryOp::Div)` |
| 59 | `Op::Binary(BinaryOp::Rem)` |
| 60 | `Op::Binary(BinaryOp::Eq)` |
| 61 | `Op::Binary(BinaryOp::Ne)` |
| 62 | `Op::Binary(BinaryOp::Lt)` |
| 63 | `Op::Binary(BinaryOp::Le)` |
| 64 | `Op::Binary(BinaryOp::Gt)` |
| 65 | `Op::Binary(BinaryOp::Ge)` |
| 66 | `Op::Binary(BinaryOp::And)` |
| 67 | `Op::Binary(BinaryOp::Or)` |
| 68 | `Op::Binary(BinaryOp::Contem)` |
| 69 | `Op::ShortCircuit { target, value: false }` |
| 70 | `Op::ShortCircuit { target, value: true }` |
| 71 | `Op::Call { argc, name }` |

Esses números são independentes dos discriminantes Rust. Mudanças incompatíveis de layout, operandos ou semântica de instruções exigem uma revisão de versão. O loader atual aceita somente v1 e os recursos 3. A resolução de bibliotecas nativas continua sendo responsabilidade do runtime instalado.

## Validação e limites do leitor desktop

A validação verifica todas as referências a tabelas e os metadados de loops/funções. Para os caminhos alcançáveis, propaga alturas de valores/destinos, escopos, loops e handlers. Rejeita underflow, joins incompatíveis, slots fora do escopo da função, saídas sem término, estruturas de controle inválidas e ciclos sem checkpoint. Exceções, `pare` e `continue` têm arestas próprias de restauração. Código só chega à VM após essa verificação.

O perfil atual limita arquivo a 16 MiB, soma de elementos a 250.000, aninhamento de protótipos a 64, orçamento contabilizado de estruturas decodificadas a 64 MiB e trabalho do verificador a 4.000.000 unidades de estado. Contagens são confrontadas com os bytes restantes antes de reservar vetores. As leituras de arquivo são limitadas. Os limites são deste leitor, não novos limites da sintaxe Expressa nem uma garantia de heap total do processo: metadados do alocador, temporários, verificação e múltiplos módulos podem consumir memória adicional.

`workspace_root`, limite de matrizes e prazo cooperativo continuam disponíveis na execução. O prazo de execução começa após a carga, assim como o parsing é anterior ao prazo no caminho `.lep`. O loader não é um sandbox de capacidades: um arquivo aceito executa com os mesmos serviços do runtime. Separação de hosts, heap global, fonte opcional/flash e validação no Pico continuam pendentes.

## Testes e medições

Há fixture v1 independente de otimizações do compilador, roundtrip de todos os opcodes, decimais exatos, Unicode/CRLF, arquivos truncados/corrompidos e mutações com checksum válida. A paridade AST/bytecode agora inclui execução carregada de `.exb` para os casos de fonte válidos. A CLI é testada sem o `.lep` original, com argumentos, saída não zero, marker, depuração e módulos relativos.

[Relatório de implementação e comparação de CPU/RSS](../benchmarks/EXB-V1-REVIEW.md).
