use crate::models::error::{AppError, AppResult};
use keyring::{Entry, Error as KeyringError};

const KEYRING_SERVICE: &str = "traceforge.runtime.default";

#[derive(Debug, Clone)]
pub enum ResolvedSchemaRegistryAuth {
    Basic { username: String, password: String },
    Bearer { token: String },
}

#[derive(Debug, Clone)]
pub enum ResolvedKafkaAuth {
    SaslPlain {
        username: String,
        password: String,
    },
    SaslScram {
        username: String,
        password: String,
        mechanism: String,
    },
}

impl ResolvedSchemaRegistryAuth {
    pub fn apply_async(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self {
            Self::Basic { username, password } => builder.basic_auth(username, Some(password)),
            Self::Bearer { token } => builder.bearer_auth(token),
        }
    }

    pub fn apply_blocking(
        &self,
        builder: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        match self {
            Self::Basic { username, password } => builder.basic_auth(username, Some(password)),
            Self::Bearer { token } => builder.bearer_auth(token),
        }
    }
}

pub fn store_runtime_secret(credential_ref: &str, secret: &str) -> AppResult<()> {
    let credential_ref = normalize_credential_ref(credential_ref)?;
    let entry = Entry::new(KEYRING_SERVICE, &account_for(credential_ref))
        .map_err(map_keyring_init_error)?;

    entry
        .set_password(secret)
        .map_err(map_keyring_runtime_error)
}

pub fn resolve_schema_registry_auth(
    auth_mode: &str,
    credential_ref: Option<&str>,
    secret_override: Option<&str>,
) -> AppResult<Option<ResolvedSchemaRegistryAuth>> {
    if auth_mode == "none" {
        return Ok(None);
    }

    let credential_ref = credential_ref
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Validation(format!(
                "credentialRef is required for auth mode '{auth_mode}'"
            ))
        })?;

    let secret = if let Some(secret_override) = secret_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        secret_override.to_string()
    } else {
        load_runtime_secret(credential_ref)?
    };

    match auth_mode {
        "basic" => {
            let (username, password) = secret.split_once(':').ok_or_else(|| {
                AppError::Validation(
                    "Basic Auth secret must be stored as 'username:password' under the referenced credential".to_string(),
                )
            })?;

            if username.trim().is_empty() || password.is_empty() {
                return Err(AppError::Validation(
                    "Basic Auth secret must include a non-empty username and password".to_string(),
                ));
            }

            Ok(Some(ResolvedSchemaRegistryAuth::Basic {
                username: username.to_string(),
                password: password.to_string(),
            }))
        }
        "bearer" => {
            if secret.trim().is_empty() {
                return Err(AppError::Validation(
                    "Bearer token secret cannot be empty".to_string(),
                ));
            }

            Ok(Some(ResolvedSchemaRegistryAuth::Bearer { token: secret }))
        }
        other => Err(AppError::Validation(format!(
            "unsupported schema registry auth mode '{other}'"
        ))),
    }
}

pub fn resolve_kafka_auth(
    auth_mode: &str,
    credential_ref: Option<&str>,
    secret_override: Option<&str>,
) -> AppResult<Option<ResolvedKafkaAuth>> {
    if auth_mode == "none" || auth_mode == "mtls" {
        return Ok(None);
    }

    let credential_ref = credential_ref
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Validation(format!(
                "credentialRef is required for auth mode '{auth_mode}'"
            ))
        })?;

    let secret = if let Some(secret_override) = secret_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        secret_override.to_string()
    } else {
        load_runtime_secret(credential_ref)?
    };

    let (username, password) = secret.split_once(':').ok_or_else(|| {
        AppError::Validation(
            "Kafka SASL secret must be stored as 'username:password' under the referenced credential".to_string(),
        )
    })?;

    if username.trim().is_empty() || password.is_empty() {
        return Err(AppError::Validation(
            "Kafka SASL secret must include a non-empty username and password".to_string(),
        ));
    }

    match auth_mode {
        "sasl-plain" => Ok(Some(ResolvedKafkaAuth::SaslPlain {
            username: username.to_string(),
            password: password.to_string(),
        })),
        "sasl-scram" => Ok(Some(ResolvedKafkaAuth::SaslScram {
            username: username.to_string(),
            password: password.to_string(),
            mechanism: "SCRAM-SHA-512".to_string(),
        })),
        other => Err(AppError::Validation(format!(
            "unsupported kafka auth mode '{other}'"
        ))),
    }
}

fn load_runtime_secret(credential_ref: &str) -> AppResult<String> {
    let credential_ref = normalize_credential_ref(credential_ref)?;

    #[cfg(test)]
    if let Some(secret) = load_test_runtime_secret(credential_ref) {
        return Ok(secret);
    }

    let entry = Entry::new(KEYRING_SERVICE, &account_for(credential_ref))
        .map_err(map_keyring_init_error)?;

    entry.get_password().map_err(map_keyring_runtime_error)
}

#[cfg(test)]
fn load_test_runtime_secret(credential_ref: &str) -> Option<String> {
    std::env::var(test_secret_env_key(credential_ref)).ok()
}

#[cfg(test)]
fn test_secret_env_key(credential_ref: &str) -> String {
    let normalized = credential_ref
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    format!("TRACEFORGE_TEST_SECRET_{normalized}")
}

fn account_for(credential_ref: &str) -> String {
    format!("credential-ref:{credential_ref}")
}

fn normalize_credential_ref(credential_ref: &str) -> AppResult<&str> {
    let trimmed = credential_ref.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "credentialRef must not be empty".to_string(),
        ));
    }
    Ok(trimmed)
}

fn map_keyring_init_error(error: KeyringError) -> AppError {
    AppError::Internal(format!(
        "failed to initialize system keyring entry: {error}"
    ))
}

fn map_keyring_runtime_error(error: KeyringError) -> AppError {
    match error {
        KeyringError::NoEntry => AppError::NotFound(
            "no secret was found in the system keyring for the requested credentialRef".to_string(),
        ),
        KeyringError::BadEncoding(_) => AppError::Validation(
            "stored secret is not valid UTF-8 for the requested credentialRef".to_string(),
        ),
        KeyringError::Invalid(_, _) | KeyringError::TooLong(_, _) => AppError::Validation(format!(
            "invalid credentialRef for system keyring lookup: {error}"
        )),
        other => AppError::Internal(format!("system keyring operation failed: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_schema_registry_auth;

    #[test]
    fn parses_basic_auth_override_secret() {
        let resolved =
            resolve_schema_registry_auth("basic", Some("registry-admin"), Some("alice:secret"))
                .expect("basic auth resolution should succeed")
                .expect("basic auth should resolve a value");

        match resolved {
            super::ResolvedSchemaRegistryAuth::Basic { username, password } => {
                assert_eq!(username, "alice");
                assert_eq!(password, "secret");
            }
            other => panic!("expected basic auth, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_basic_auth_override_secret() {
        let error =
            resolve_schema_registry_auth("basic", Some("registry-admin"), Some("alice-only"))
                .expect_err("malformed basic secret should fail");

        assert!(error.to_string().contains("username:password"));
    }

    #[test]
    fn parses_bearer_override_secret() {
        let resolved =
            resolve_schema_registry_auth("bearer", Some("registry-token"), Some("token-value"))
                .expect("bearer auth resolution should succeed")
                .expect("bearer auth should resolve a value");

        match resolved {
            super::ResolvedSchemaRegistryAuth::Bearer { token } => {
                assert_eq!(token, "token-value");
            }
            other => panic!("expected bearer auth, got {other:?}"),
        }
    }
}
