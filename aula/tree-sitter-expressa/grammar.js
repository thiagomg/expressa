/// Tree-sitter grammar for Expressa (keywords, comments, strings, numbers).
module.exports = grammar({
  name: "expressa",

  extras: $ => [/\s/, $.comment],

  word: $ => $.identifier,

  rules: {
    source_file: $ => repeat($._item),

    _item: $ => choice(
      $.keyword,
      $.key_ident,
      $.identifier,
      $.string,
      $.number,
      $.punctuation,
    ),

    comment: _ => token(choice(
      seq("//", /.*/),
      seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/"),
    )),

    string: _ => token(seq('"', repeat(choice(/[^"\\\n]/, /\\./)), '"')),

    number: _ => token(/\d+(?:[.,]\d+)?/),

    keyword: _ => choice(
      "se",
      "senao",
      "senão",
      "ou",
      "e",
      "nao",
      "não",
      "inicio",
      "início",
      "fim",
      "funcao",
      "função",
      "para",
      "de",
      "ate",
      "até",
      "em",
      "repita",
      "enquanto",
      "vezes",
      "importe",
      "contem",
      "contém",
      "verdadeiro",
      "falso",
      "se_falhar",
      "retorne",
      "pare",
      "continue",
      "continua",
    ),

    key_ident: _ => token(/:[\p{L}_][\p{L}\p{N}_]*/u),

    identifier: _ => token(/[\p{L}_][\p{L}\p{N}_]*/u),

    punctuation: _ => choice(
      "{", "}", "(", ")", "[", "]", ",", "=", ".", ":",
      "+", "-", "*", "/", "%", "<", ">", "!",
    ),
  },
});
