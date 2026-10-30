use std::path::{Component, Path, PathBuf};

/// Project used when a request names none: files from before projects
/// existed live here (see `projetos::preparar`).
pub const PROJETO_PADRAO: &str = "principal";

/// One path segment: ASCII letters, digits, `_`, `-`. Never `.` or `..`,
/// and never a hidden name, so it cannot clash with the server's own files.
fn nome_valido(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Allowed student folder names: ASCII letters, digits, `_`, `-`.
pub fn sanitize_student(student: &str) -> Result<&str, String> {
    let name = if student.trim().is_empty() {
        "local"
    } else {
        student.trim()
    };
    if !nome_valido(name) {
        return Err(format!("nome de aluno inválido: {student:?}"));
    }
    Ok(name)
}

/// Allowed project names: same as students. Empty is [`PROJETO_PADRAO`].
pub fn sanitize_project(project: &str) -> Result<&str, String> {
    let name = project.trim();
    if name.is_empty() {
        return Ok(PROJETO_PADRAO);
    }
    if !nome_valido(name) {
        return Err(format!(
            "nome de projeto inválido: {project:?} (use letras, números, _ e -)"
        ));
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

/// `{root}/{student}/{project}/{rel}` if `rel` stays inside the project.
pub fn student_file(
    root: &Path,
    student: &str,
    project: &str,
    rel: &str,
) -> Result<PathBuf, String> {
    let base = project_dir(root, student, project)?;
    let rel = rel.trim();
    if rel.is_empty() {
        return Err("caminho vazio".into());
    }
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err("caminho absoluto não é permitido".into());
    }
    let full = lexical_normalize(&base.join(rel_path));
    if !full.starts_with(&base) {
        return Err(format!("caminho fora da pasta do projeto: {rel}"));
    }
    // `.` or `a/..` is the project folder itself: deleting or renaming it
    // would take every file of the project.
    if full == lexical_normalize(&base) {
        return Err(format!("caminho é a própria pasta do projeto: {rel}"));
    }
    Ok(full)
}

pub fn student_dir(root: &Path, student: &str) -> Result<PathBuf, String> {
    let student = sanitize_student(student)?;
    Ok(root.join(student))
}

/// `{root}/{student}/{project}`.
pub fn project_dir(root: &Path, student: &str, project: &str) -> Result<PathBuf, String> {
    let project = sanitize_project(project)?;
    Ok(student_dir(root, student)?.join(project))
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
        for rel in [
            "../bruno/segredo.lep",
            "../../bruno/principal/a.lep",
            "../outro/a.lep",
        ] {
            let err = student_file(root, "ana", "aula-1", rel).unwrap_err();
            assert!(err.contains("fora da pasta"), "{rel}: {err}");
        }
    }

    #[test]
    fn project_names_are_one_plain_segment() {
        let root = Path::new("/var/aula");
        for bad in ["..", ".", "a/b", "../bruno", ".escondido", "com espaço"] {
            assert!(student_file(root, "ana", bad, "a.lep").is_err(), "{bad}");
        }
        let p = student_file(root, "ana", "trabalho-final", "a.lep").unwrap();
        assert_eq!(p, PathBuf::from("/var/aula/ana/trabalho-final/a.lep"));
    }

    #[test]
    fn empty_project_is_principal() {
        let p = student_file(Path::new("/var/aula"), "ana", "", "a.lep").unwrap();
        assert_eq!(p, PathBuf::from("/var/aula/ana/principal/a.lep"));
    }

    #[test]
    fn rejects_the_student_folder_itself() {
        let root = Path::new("/var/aula");
        for rel in [".", "./", "lib/..", "./lib/../."] {
            let err = student_file(root, "ana", "aula-1", rel).unwrap_err();
            assert!(err.contains("própria pasta"), "{rel}: {err}");
        }
    }

    #[test]
    fn rejects_absolute() {
        assert!(student_file(Path::new("/var/aula"), "ana", "", "/etc/passwd").is_err());
    }

    #[test]
    fn accepts_nested_lep() {
        let p = student_file(Path::new("/var/aula"), "ana", "aula-1", "ex/bhaskara.lep").unwrap();
        assert_eq!(p, PathBuf::from("/var/aula/ana/aula-1/ex/bhaskara.lep"));
    }

    #[test]
    fn empty_student_is_local() {
        let p = student_file(Path::new("/var/aula"), "", "", "a.lep").unwrap();
        assert_eq!(p, PathBuf::from("/var/aula/local/principal/a.lep"));
    }
}
