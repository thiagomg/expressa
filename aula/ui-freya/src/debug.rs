//! Debugger panel model: breakpoints, pause snapshot, watches.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variavel {
    pub escopo: String,
    pub nome: String,
    pub valor: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quadro {
    pub nome: String,
    pub arquivo: String,
    pub linha: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Depuracao {
    pub visivel: bool,
    pub pausado: bool,
    pub pontos: Vec<(String, u32)>,
    pub observados: Vec<String>,
    pub vars: Vec<Variavel>,
    pub pilha: Vec<Quadro>,
    pub arquivo: String,
    pub linha: u32,
    pub trecho: String,
    pub erro: Option<String>,
}

impl Default for Depuracao {
    fn default() -> Self {
        Self {
            visivel: true,
            pausado: false,
            pontos: Vec::new(),
            observados: Vec::new(),
            vars: Vec::new(),
            pilha: Vec::new(),
            arquivo: String::new(),
            linha: 0,
            trecho: String::new(),
            erro: None,
        }
    }
}

impl Depuracao {
    /// Returns whether the breakpoint is now set.
    pub fn alternar_ponto(&mut self, arquivo: String, linha: u32) -> bool {
        if let Some(i) = self
            .pontos
            .iter()
            .position(|(f, l)| f == &arquivo && *l == linha)
        {
            self.pontos.remove(i);
            false
        } else {
            self.pontos.push((arquivo, linha));
            self.pontos.sort();
            true
        }
    }

    pub fn adicionar_observado(&mut self, nome: &str) {
        let nome = nome.trim();
        if nome.is_empty() || self.observados.iter().any(|n| n == nome) {
            return;
        }
        self.observados.push(nome.to_string());
    }

    pub fn remover_observado(&mut self, nome: &str) {
        self.observados.retain(|n| n != nome);
    }

    pub fn aplicar_pausa(
        &mut self,
        arquivo: String,
        linha: u32,
        trecho: String,
        vars: Vec<Variavel>,
        pilha: Vec<Quadro>,
        erro: Option<String>,
    ) {
        self.pausado = true;
        self.visivel = true;
        self.arquivo = arquivo;
        self.linha = linha;
        self.trecho = trecho;
        self.vars = vars;
        self.pilha = pilha;
        self.erro = erro;
    }

    pub fn fim(&mut self) {
        self.pausado = false;
        self.vars.clear();
        self.pilha.clear();
        self.arquivo.clear();
        self.linha = 0;
        self.trecho.clear();
        self.erro = None;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarcaGutter {
    Nada,
    Ponto,
    Atual,
    PontoAtual,
}

pub fn marca_gutter(
    pontos: &[(String, u32)],
    arquivo: &str,
    linha: u32,
    pausado: bool,
    pausa_arquivo: &str,
    pausa_linha: u32,
) -> MarcaGutter {
    let ponto = pontos.iter().any(|(f, l)| f == arquivo && *l == linha);
    let atual = pausado && pausa_linha == linha && (pausa_arquivo.is_empty() || pausa_arquivo == arquivo);
    match (ponto, atual) {
        (true, true) => MarcaGutter::PontoAtual,
        (true, false) => MarcaGutter::Ponto,
        (false, true) => MarcaGutter::Atual,
        (false, false) => MarcaGutter::Nada,
    }
}

pub fn valor_observado(nome: &str, vars: &[Variavel]) -> String {
    vars.iter()
        .find(|v| v.nome == nome)
        .map(|v| v.valor.clone())
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alterna_ponto() {
        let mut d = Depuracao::default();
        assert!(d.alternar_ponto("a.lep".into(), 3));
        assert_eq!(d.pontos, [("a.lep".into(), 3)]);
        assert!(!d.alternar_ponto("a.lep".into(), 3));
        assert!(d.pontos.is_empty());
    }

    #[test]
    fn marca_gutter_ponto_e_pausa() {
        let pts = vec![("a.lep".into(), 2)];
        assert_eq!(
            marca_gutter(&pts, "a.lep", 2, true, "a.lep", 2),
            MarcaGutter::PontoAtual
        );
        assert_eq!(
            marca_gutter(&pts, "a.lep", 2, false, "a.lep", 2),
            MarcaGutter::Ponto
        );
        assert_eq!(
            marca_gutter(&pts, "a.lep", 1, true, "a.lep", 1),
            MarcaGutter::Atual
        );
        assert_eq!(
            marca_gutter(&pts, "b.lep", 2, true, "a.lep", 2),
            MarcaGutter::Nada
        );
    }

    #[test]
    fn observado_mostra_traco_sem_variavel() {
        let mut d = Depuracao::default();
        d.adicionar_observado("x");
        d.adicionar_observado("x");
        assert_eq!(d.observados, ["x"]);
        assert_eq!(valor_observado("x", &[]), "—");
        let vars = vec![Variavel {
            escopo: "módulo".into(),
            nome: "x".into(),
            valor: "1".into(),
        }];
        assert_eq!(valor_observado("x", &vars), "1");
    }
}
