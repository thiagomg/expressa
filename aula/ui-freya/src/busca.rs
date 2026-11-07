//! Search in a buffer (Ctrl+F).

/// Next occurrence of `termo` at or after `de` (byte offset). Case-sensitive.
pub fn proxima(texto: &str, termo: &str, de: usize) -> Option<usize> {
    if termo.is_empty() {
        return None;
    }
    let de = de.min(texto.len());
    texto[de..].find(termo).map(|i| de + i)
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
}
