//! Persistent project identity, independent of display names and run directories.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectIdentity {
    Local {
        root: PathBuf,
    },
    Remote {
        endpoint: String,
        student: String,
        project: String,
    },
}

impl ProjectIdentity {
    pub fn key(&self) -> String {
        // Structured encoding keeps punctuation and field boundaries unambiguous.
        expressa_aula_storage::revision(&serde_json::to_vec(self).expect("serializable identity"))
            .trim_start_matches("sha256:")
            .to_string()
    }

    pub fn label(&self) -> String {
        match self {
            Self::Local { root } => root.display().to_string(),
            Self::Remote {
                endpoint,
                student,
                project,
            } => format!("{endpoint} — {student}/{project}"),
        }
    }
}

pub fn normalize_endpoint(url: &str) -> Result<String, String> {
    let endpoint = tonic::transport::Endpoint::from_shared(url.trim().to_string())
        .map_err(|e| e.to_string())?;
    let uri = endpoint.uri();
    let scheme = uri
        .scheme_str()
        .ok_or("servidor sem protocolo")?
        .to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err("protocolo do servidor inválido".into());
    }
    let host = uri
        .host()
        .ok_or("servidor sem endereço")?
        .to_ascii_lowercase();
    if uri.authority().is_some_and(|a| a.as_str().contains('@')) || uri.query().is_some() {
        return Err("endereço do servidor não pode conter credenciais ou consulta".into());
    }
    let port = match uri.port_u16() {
        Some(80) if scheme == "http" => String::new(),
        Some(443) if scheme == "https" => String::new(),
        Some(port) => format!(":{port}"),
        None => String::new(),
    };
    Ok(format!(
        "{scheme}://{host}{port}{}",
        if uri.path() == "/" { "" } else { uri.path() }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoints_and_structured_fields_do_not_collide() {
        assert_eq!(
            normalize_endpoint("http://EXAMPLE.com:80/").unwrap(),
            "http://example.com"
        );
        assert_eq!(
            normalize_endpoint("https://EXAMPLE.com:443/").unwrap(),
            "https://example.com"
        );
        assert_ne!(
            normalize_endpoint("http://server/api").unwrap(),
            normalize_endpoint("http://server/api/").unwrap()
        );
        assert_eq!(
            normalize_endpoint("http://[::1]:80/").unwrap(),
            "http://[::1]"
        );
        let id = |endpoint: &str, student: &str, project: &str| ProjectIdentity::Remote {
            endpoint: normalize_endpoint(endpoint).unwrap(),
            student: student.into(),
            project: project.into(),
        };
        assert_eq!(
            id("http://SERVER:80/", "a", "b"),
            id("http://server", "a", "b")
        );
        assert_ne!(
            id("http://server:3001", "a", "b").key(),
            id("http://server:3002", "a", "b").key()
        );
        assert_ne!(
            id("http://server", "a-b", "c").key(),
            id("http://server", "a", "b-c").key()
        );
        assert_ne!(
            id("http://server", "a", "b.c").key(),
            id("http://server", "a", "b_c").key()
        );
    }
}
