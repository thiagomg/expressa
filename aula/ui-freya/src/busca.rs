//! Search in a buffer (Ctrl+F).

/// Next occurrence of `termo` at or after `de` (byte offset). Case-sensitive.
pub fn proxima(texto: &str, termo: &str, de: usize) -> Option<usize> {
    if termo.is_empty() {
        return None;
    }
    let de = de.min(texto.len());
    texto[de..].find(termo).map(|i| de + i)
}

pub fn substituir_todas(texto: &str, termo: &str, novo: &str) -> String {
    if termo.is_empty() {
        return texto.to_string();
    }
    texto.replace(termo, novo)
}

pub fn anterior(texto: &str, termo: &str, de: usize) -> Option<usize> {
    if termo.is_empty() {
        return None;
    }
    let de = de.min(texto.len());
    texto[..de].rfind(termo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acha_identificador_com_acento() {
        let t = "função não\nfunção\n";
        assert_eq!(proxima(t, "função", 0), Some(0));
        assert_eq!(proxima(t, "função", 1), Some("função não\n".len()));
        assert_eq!(anterior(t, "não", t.len()), Some("função ".len()));
        assert_eq!(proxima(t, "xyz", 0), None);
    }

    #[test]
    fn substitui_todas_as_ocorrencias() {
        assert_eq!(substituir_todas("ola ola", "ola", "olá"), "olá olá");
        assert_eq!(substituir_todas("ola", "", "x"), "ola");
    }
}
