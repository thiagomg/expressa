use std::path::{Component, Path, PathBuf};

/// Allowed student folder names: ASCII letters, digits, `_`, `-`.
pub fn sanitize_student(student: &str) -> Result<&str, String> {
    let name = if student.trim().is_empty() {
        "local"
    } else {
        student.trim()
    };
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!("nome de aluno inválido: {student:?}"));
    }
    Ok(name)
}

pub fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(Component::RootDir),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

/// `{root}/{student}/{rel}` if `rel` stays inside the student folder.
pub fn student_file(root: &Path, student: &str, rel: &str) -> Result<PathBuf, String> {
    let student = sanitize_student(student)?;
    let rel = rel.trim();
    if rel.is_empty() {
        return Err("caminho vazio".into());
    }
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err("caminho absoluto não é permitido".into());
    }
    let base = root.join(student);
    let full = lexical_normalize(&base.join(rel_path));
    if !full.starts_with(&base) {
        return Err(format!("caminho fora da pasta do aluno: {rel}"));
    }
    Ok(full)
}

pub fn student_dir(root: &Path, student: &str) -> Result<PathBuf, String> {
    let student = sanitize_student(student)?;
    Ok(root.join(student))
}

pub fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_dir_escape() {
        let root = Path::new("/var/aula");
        let err = student_file(root, "ana", "../bruno/segredo.lep").unwrap_err();
        assert!(err.contains("fora da pasta"), "{err}");
    }

    #[test]
    fn rejects_absolute() {
        assert!(student_file(Path::new("/var/aula"), "ana", "/etc/passwd").is_err());
    }

    #[test]
    fn accepts_nested_lep() {
        let p = student_file(Path::new("/var/aula"), "ana", "ex/bhaskara.lep").unwrap();
        assert_eq!(p, PathBuf::from("/var/aula/ana/ex/bhaskara.lep"));
    }

    #[test]
    fn empty_student_is_local() {
        let p = student_file(Path::new("/var/aula"), "", "a.lep").unwrap();
        assert_eq!(p, PathBuf::from("/var/aula/local/a.lep"));
    }
}
