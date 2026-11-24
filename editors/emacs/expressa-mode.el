;;; expressa-mode.el --- Major mode for Expressa (.lep)  -*- lexical-binding: t; -*-

;; SPDX-License-Identifier: MIT

;;; Commentary:
;;
;; Highlight and indentation for Expressa.
;;
;;   (add-to-list 'load-path "/caminho/para/expressa/editors/emacs")
;;   (require 'expressa-mode)
;;
;; TAB indents; Shift+TAB (backtab) unindents the line or the region.

;;; Code:

(defconst expressa-keywords
  '("se" "senao" "senão" "ou" "e" "nao" "não" "inicio" "início" "fim"
    "funcao" "função" "para" "em" "repita" "enquanto" "vezes" "importe"
    "contem" "contém" "se_falhar" "retorne" "pare" "continue"))

(defconst expressa-builtins
  '("adicione" "adicione_arquivo" "agora" "ajuda" "aleatorio" "aleatório"
    "altura" "ambiente" "apague" "apague_arquivo" "apague_linha"
    "apague_pasta" "arcocosseno" "arcosseno" "arcotangente" "arcotangente2"
    "argumentos" "arredonde" "avalie" "bloco" "carregue" "chaves_de" "cheia"
    "circulo" "clone" "colunas" "comeca_com" "começa_com" "como_imagem"
    "como_texto" "conjunto" "copie" "cor" "cos" "cosseno" "crie_pasta"
    "círculo" "data" "det" "dias_entre" "documento" "durma" "eh_bissexto"
    "eh_infinito" "eh_pasta" "eh_terminal" "eixos" "escreva" "escreva_em"
    "escreva_erro" "evento" "execute" "existe" "exp" "extensao" "extensão"
    "falhe" "formate" "formate_data" "formato" "funcoes" "fundo" "funções"
    "garanta" "grafico" "graus" "gráfico" "hoje" "identidade" "imagem"
    "junte" "junte_caminho" "laco_grafico" "largura" "laço_gráfico" "leia"
    "leia_arquivo" "leia_bytes" "leia_csv" "leia_linhas" "limpe" "limpe_tela"
    "linha" "linhas" "linhas_de" "liste" "log" "log10" "maiuscula"
    "maiúscula" "mapa" "matriz" "minuscula" "minúscula" "modulos" "mostre"
    "mova" "módulos" "negrito" "nome_do_arquivo" "nova" "nova_linha" "numero"
    "número" "ordene" "pare_laco" "pare_laço" "pasta_atual" "pasta_de" "pi"
    "pinte" "piso" "pixel" "plote" "ponto" "posicao_cursor" "posição_cursor"
    "potencia" "potência" "primeiro" "procure" "quadro" "radianos" "raiz"
    "redimensione" "remova" "retangulo" "retângulo" "rotacione" "saia"
    "salve" "salve_arquivo" "salve_csv" "sem_acento" "semente" "sen" "seno"
    "separe" "substitua" "tamanho" "tamanho_tela" "tangente" "termina_com"
    "teto" "texto" "texto_utf8" "tipo" "tipos" "transposta" "ultimo" "uns"
    "valores_de" "xy" "zeros" "é_bissexto" "é_infinito" "é_pasta"
    "é_terminal" "último"))

(defconst expressa-constants
  '("verdadeiro" "falso" "nada"))

(defconst expressa-font-lock-keywords
  `(("^#!.*" . font-lock-comment-face)
    ("^///.*" 0 font-lock-doc-face t)
    (,(regexp-opt expressa-keywords 'symbols) . font-lock-keyword-face)
    (,(regexp-opt expressa-builtins 'symbols) . font-lock-builtin-face)
    (,(regexp-opt expressa-constants 'symbols) . font-lock-constant-face)
    (":[A-Za-zÀ-ÿ_][A-Za-zÀ-ÿ0-9_]*" . font-lock-string-face)
    ("\\_<[0-9][0-9_]*\\(?:\\.[0-9_]+\\)?\\_>"
     . ,(if (facep 'font-lock-number-face)
            'font-lock-number-face
          'font-lock-constant-face))))

(defcustom expressa-indent-offset 4
  "Indentation step for Expressa blocks (`inicio'/`fim', `{`/`}`, `[`/`]`, `(`/`)`)."
  :type 'integer
  :group 'expressa)

(defvar expressa-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?_ "w" table)
    (modify-syntax-entry ?/ ". 124b" table)
    (modify-syntax-entry ?* ". 23" table)
    (modify-syntax-entry ?\n "> b" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?\\ "\\" table)
    (modify-syntax-entry ?{ "(}" table)
    (modify-syntax-entry ?} "){" table)
    (modify-syntax-entry ?[ "(]" table)
    (modify-syntax-entry ?] ")[" table)
    (modify-syntax-entry ?( "()" table)
    (modify-syntax-entry ?) ")(" table)
    table)
  "Syntax table for `expressa-mode'.")

(defvar expressa-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map (kbd "<backtab>") #'expressa-indent-shift-left)
    (define-key map (kbd "S-<tab>") #'expressa-indent-shift-left)
    (define-key map (kbd "S-TAB") #'expressa-indent-shift-left)
    map)
  "Keymap for `expressa-mode'.")

(defun expressa--opens-closes (start end)
  "Count block openers and closers between START and END."
  (save-excursion
    (goto-char start)
    (let ((opens 0)
          (closes 0))
      (while (< (point) end)
        (if (nth 8 (syntax-ppss))
            (forward-char 1)
          (cond
           ((looking-at "\\_<\\(inicio\\|início\\)\\_>")
            (setq opens (1+ opens))
            (goto-char (match-end 0)))
           ((looking-at "\\_<fim\\_>")
            (setq closes (1+ closes))
            (goto-char (match-end 0)))
           ((memq (char-after) '(?{ ?[ ?\())
            (setq opens (1+ opens))
            (forward-char 1))
           ((memq (char-after) '(?} ?] ?\)))
            (setq closes (1+ closes))
            (forward-char 1))
           (t (forward-char 1)))))
      (cons opens closes))))

(defun expressa--line-starts-with-closer ()
  "Non-nil if this line begins with `fim', `}', `]' or `)' (after whitespace)."
  (save-excursion
    (beginning-of-line)
    (skip-chars-forward " \t")
    (or (memq (char-after) '(?} ?] ?\)))
        (looking-at "\\_<fim\\_>"))))

(defun expressa--calculate-indent ()
  "Indent in columns for the current line."
  (save-excursion
    (let ((bol (line-beginning-position))
          (depth 0))
      (goto-char (point-min))
      (while (< (point) bol)
        (let* ((ls (line-beginning-position))
               (le (line-end-position))
               (oc (expressa--opens-closes ls le)))
          (setq depth (max 0 (+ depth (car oc) (- (cdr oc))))))
        (forward-line 1))
      (when (expressa--line-starts-with-closer)
        (setq depth (max 0 (1- depth))))
      (* depth expressa-indent-offset))))

(defun expressa-indent-line ()
  "Indent current line as Expressa code."
  (interactive)
  (let* ((offset (- (point)
                    (save-excursion
                      (back-to-indentation)
                      (point))))
         (indent (expressa--calculate-indent)))
    (indent-line-to indent)
    (when (> offset 0)
      (goto-char (min (line-end-position) (+ (point) offset))))))

(defun expressa-indent-region (start end)
  "Indent each line between START and END."
  (save-excursion
    (setq end (copy-marker end))
    (goto-char start)
    (forward-line 0)
    (while (< (point) end)
      (unless (looking-at-p "[ \t]*$")
        (expressa-indent-line))
      (forward-line 1))))

(defun expressa-indent-shift-left (start end)
  "Unindent the current line or region by `expressa-indent-offset'."
  (interactive
   (if (use-region-p)
       (list (region-beginning) (region-end))
     (list (line-beginning-position) (line-end-position))))
  (indent-rigidly start end (- expressa-indent-offset)))

;;;###autoload
(define-derived-mode expressa-mode prog-mode "Expressa"
  "Major mode for Expressa source files (`.lep')."
  :syntax-table expressa-mode-syntax-table
  (setq-local comment-start "// ")
  (setq-local comment-end "")
  (setq-local comment-start-skip "//+\\s-*")
  (setq-local comment-multi-line t)
  (setq-local font-lock-defaults '(expressa-font-lock-keywords))
  (setq-local indent-line-function #'expressa-indent-line)
  (setq-local indent-region-function #'expressa-indent-region)
  (setq-local indent-tabs-mode nil)
  (setq-local tab-width expressa-indent-offset)
  (setq-local electric-indent-chars (append '(?} ?] ?\)) electric-indent-chars)))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.lep\\'" . expressa-mode))

(provide 'expressa-mode)

;;; expressa-mode.el ends here
