use std::fmt;

#[derive(Debug, Clone)]
pub enum AuthMethod {
    CurrentUser,
    Credentials {
        username: String,
        password: String,
        domain: Option<String>,
    },
    NTHash {
        username: String,
        nt_hash: String,
        domain: Option<String>,
    },
    Kerberos {
        principal: String,
        realm: String,
    },
}

impl AuthMethod {
    pub fn current_user() -> Self {
        AuthMethod::CurrentUser
    }

    pub fn with_credentials(username: &str, password: &str, domain: Option<&str>) -> Self {
        AuthMethod::Credentials {
            username: username.to_string(),
            password: password.to_string(),
            domain: domain.map(|d| d.to_string()),
        }
    }

    pub fn with_nt_hash(username: &str, nt_hash: &str, domain: Option<&str>) -> Self {
        AuthMethod::NTHash {
            username: username.to_string(),
            nt_hash: nt_hash.to_string(),
            domain: domain.map(|d| d.to_string()),
        }
    }

    pub fn kerberos(principal: &str, realm: &str) -> Self {
        AuthMethod::Kerberos {
            principal: principal.to_string(),
            realm: realm.to_string(),
        }
    }

    pub fn username(&self) -> Option<String> {
        match self {
            AuthMethod::CurrentUser => None,
            AuthMethod::Credentials { username, .. } => Some(username.clone()),
            AuthMethod::NTHash { username, .. } => Some(username.clone()),
            AuthMethod::Kerberos { principal, .. } => Some(principal.clone()),
        }
    }

    pub fn domain(&self) -> Option<String> {
        match self {
            AuthMethod::CurrentUser => None,
            AuthMethod::Credentials { domain, .. } => domain.clone(),
            AuthMethod::NTHash { domain, .. } => domain.clone(),
            AuthMethod::Kerberos { realm, .. } => Some(realm.clone()),
        }
    }
}

impl fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMethod::CurrentUser => write!(f, "CurrentUser"),
            AuthMethod::Credentials { username, domain, .. } => {
                if let Some(d) = domain {
                    write!(f, "{}\\{}", d, username)
                } else {
                    write!(f, "{}", username)
                }
            }
            AuthMethod::NTHash { username, domain, .. } => {
                if let Some(d) = domain {
                    write!(f, "{}\\{} (NTHash)", d, username)
                } else {
                    write!(f, "{} (NTHash)", username)
                }
            }
            AuthMethod::Kerberos { principal, realm } => {
                write!(f, "{}@{} (Kerberos)", principal, realm)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub method: AuthMethod,
    pub target_host: String,
}

impl AuthContext {
    pub fn new(method: AuthMethod, target_host: &str) -> Self {
        AuthContext {
            method,
            target_host: target_host.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_user() {
        let auth = AuthMethod::current_user();
        assert!(auth.username().is_none());
    }

    #[test]
    fn test_credentials() {
        let auth = AuthMethod::with_credentials("admin", "password123", Some("DOMAIN"));
        assert_eq!(auth.username(), Some("admin".to_string()));
        assert_eq!(auth.domain(), Some("DOMAIN".to_string()));
    }

    #[test]
    fn test_nt_hash() {
        let hash = "8846f7eaee8fb117ad06bdd830b7586c";
        let auth = AuthMethod::with_nt_hash("admin", hash, Some("DOMAIN"));
        assert_eq!(auth.username(), Some("admin".to_string()));
    }

    #[test]
    fn test_kerberos() {
        let auth = AuthMethod::kerberos("admin", "DOMAIN.COM");
        assert_eq!(auth.username(), Some("admin".to_string()));
        assert_eq!(auth.domain(), Some("DOMAIN.COM".to_string()));
    }
}
