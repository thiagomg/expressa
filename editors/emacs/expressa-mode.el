;;; expressa-mode.el --- Major mode for Expressa (.lep)  -*- lexical-binding: t; -*-

;; SPDX-License-Identifier: MIT

;;; Commentary:
;;
;; Highlight de palavras-chave, nativas, literais, comentários e strings.
;;
;;   (add-to-list 'load-path "/caminho/para/expressa/editors/emacs")
;;   (require 'expressa-mode)

;;; Code:

(defconst expressa-keywords
  '("se_falhar" "se" "senao" "senão" "ou" "e" "nao" "não"
    "inicio" "início" "fim" "funcao" "função"
    "para" "de" "ate" "até" "em" "repita" "enquanto" "vezes"
    "mapa" "matriz" "importe" "contem" "contém"))

(defconst expressa-builtins
  '("escreva" "escreva_erro" "sair" "leia" "leia_linhas" "argumentos"
    "numero" "formato" "raiz"
    "transposta" "det" "identidade"
    "tamanho" "primeiro" "ultimo"
    "maiuscula" "minuscula" "substitua" "separe" "junte" "limpe"
    "leia_arquivo" "salve_arquivo" "adicione_arquivo"
    "leia_csv" "salve_csv"))

(defconst expressa-constants
  '("verdadeiro" "falso"))

(defconst expressa-font-lock-keywords
  `(("^#!.*" . font-lock-comment-face)
    (,(regexp-opt expressa-keywords 'symbols) . font-lock-keyword-face)
    (,(regexp-opt expressa-builtins 'symbols) . font-lock-builtin-face)
    (,(regexp-opt expressa-constants 'symbols) . font-lock-constant-face)
    ("\\_<[0-9][0-9_]*\\(?:\\.[0-9_]+\\)?\\_>"
     . ,(if (facep 'font-lock-number-face)
            'font-lock-number-face
          'font-lock-constant-face))))

(defvar expressa-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?_ "w" table)
    (modify-syntax-entry ?/ ". 124b" table)
    (modify-syntax-entry ?* ". 23" table)
    (modify-syntax-entry ?\n "> b" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "\\" table)
    table)
  "Syntax table for `expressa-mode'.")

;;;###autoload
(define-derived-mode expressa-mode prog-mode "Expressa"
  "Major mode for Expressa source files (`.lep')."
  :syntax-table expressa-mode-syntax-table
  (setq-local comment-start "// ")
  (setq-local comment-end "")
  (setq-local comment-start-skip "//+\\s-*")
  (setq-local comment-multi-line t)
  (setq-local font-lock-defaults '(expressa-font-lock-keywords)))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.lep\\'" . expressa-mode))

(provide 'expressa-mode)

;;; expressa-mode.el ends here
