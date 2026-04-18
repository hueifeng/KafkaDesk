use crate::models::cluster::ClusterProfileDto;
use crate::models::error::{AppError, AppResult};
use crate::services::credentials::{resolve_kafka_auth, ResolvedKafkaAuth};
use rdkafka::ClientConfig;
use std::path::Path;

pub fn apply_kafka_read_consumer_config(
    config: &mut ClientConfig,
    profile: &ClusterProfileDto,
) -> AppResult<()> {
    apply_kafka_read_consumer_config_with_secret(config, profile, None)
}

pub(crate) fn apply_kafka_read_consumer_config_with_secret(
    config: &mut ClientConfig,
    profile: &ClusterProfileDto,
    secret_override: Option<&str>,
) -> AppResult<()> {
    config
        .set("bootstrap.servers", &profile.bootstrap_servers)
        .set("socket.timeout.ms", "5000")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false");

    apply_kafka_security_config_with_secret(config, profile, secret_override)
}

pub fn apply_kafka_security_config(
    config: &mut ClientConfig,
    profile: &ClusterProfileDto,
) -> AppResult<()> {
    apply_kafka_security_config_with_secret(config, profile, None)
}

pub(crate) fn apply_kafka_security_config_with_secret(
    config: &mut ClientConfig,
    profile: &ClusterProfileDto,
    secret_override: Option<&str>,
) -> AppResult<()> {
    let kafka_auth = resolve_kafka_auth(
        &profile.auth_mode,
        profile.auth_credential_ref.as_deref(),
        secret_override,
    )?;

    let security_protocol = match profile.auth_mode.as_str() {
        "none" => match profile.tls_mode.as_str() {
            "system-default" => None,
            "tls-required" | "tls-insecure" => Some("ssl"),
            other => {
                return Err(AppError::Validation(format!("unknown TLS mode '{other}'")));
            }
        },
        "sasl-plain" | "sasl-scram" => match profile.tls_mode.as_str() {
            "system-default" => Some("sasl_plaintext"),
            "tls-required" | "tls-insecure" => Some("sasl_ssl"),
            other => {
                return Err(AppError::Validation(format!("unknown TLS mode '{other}'")));
            }
        },
        "mtls" => Some("ssl"),
        other => {
            return Err(AppError::Validation(format!("unknown auth mode '{other}'")));
        }
    };

    if let Some(security_protocol) = security_protocol {
        config.set("security.protocol", security_protocol);
    }

    if profile.tls_mode == "tls-insecure" {
        config.set("enable.ssl.certificate.verification", "false");
    }

    if let Some(ca_path) =
        normalize_optional_file_path(profile.tls_ca_cert_path.as_deref(), "TLS CA certificate")?
    {
        config.set("ssl.ca.location", &ca_path);
    }

    if profile.auth_mode == "mtls" {
        let client_cert_path = require_file_path(
            profile.tls_client_cert_path.as_deref(),
            "TLS client certificate",
        )?;
        let client_key_path = require_file_path(
            profile.tls_client_key_path.as_deref(),
            "TLS client private key",
        )?;

        config
            .set("ssl.certificate.location", &client_cert_path)
            .set("ssl.key.location", &client_key_path);
    }

    match kafka_auth {
        Some(ResolvedKafkaAuth::SaslPlain { username, password }) => {
            config
                .set("sasl.mechanism", "PLAIN")
                .set("sasl.username", &username)
                .set("sasl.password", &password);
        }
        Some(ResolvedKafkaAuth::SaslScram {
            username,
            password,
            mechanism,
        }) => {
            config
                .set("sasl.mechanism", &mechanism)
                .set("sasl.username", &username)
                .set("sasl.password", &password);
        }
        None => {}
    }

    Ok(())
}

pub(crate) fn normalize_optional_file_path(
    path: Option<&str>,
    label: &str,
) -> AppResult<Option<String>> {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    Ok(Some(validate_file_path(path, label)?))
}

pub(crate) fn require_file_path(path: Option<&str>, label: &str) -> AppResult<String> {
    let path = path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Validation(format!("{label} path is required")))?;

    validate_file_path(path, label)
}

fn validate_file_path(path: &str, label: &str) -> AppResult<String> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Err(AppError::Path(format!(
            "{label} path does not exist: {path}"
        )));
    }
    if !file_path.is_file() {
        return Err(AppError::Path(format!(
            "{label} path is not a file: {path}"
        )));
    }

    Ok(file_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::{apply_kafka_read_consumer_config, apply_kafka_security_config_with_secret};
    use crate::models::cluster::ClusterProfileDto;
    use rdkafka::ClientConfig;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    fn sample_profile(auth_mode: &str, tls_mode: &str) -> ClusterProfileDto {
        ClusterProfileDto {
            id: "cluster-1".to_string(),
            name: "Cluster One".to_string(),
            environment: "dev".to_string(),
            bootstrap_servers: "localhost:9092".to_string(),
            auth_mode: auth_mode.to_string(),
            auth_credential_ref: None,
            tls_mode: tls_mode.to_string(),
            tls_ca_cert_path: None,
            tls_client_cert_path: None,
            tls_client_key_path: None,
            schema_registry_profile_id: None,
            notes: None,
            tags: vec![],
            is_favorite: false,
            created_at: "2026-04-18T00:00:00Z".to_string(),
            updated_at: "2026-04-18T00:00:00Z".to_string(),
            last_connected_at: None,
            is_archived: false,
        }
    }

    fn create_temp_file(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("traceforge-{name}-{}.pem", Uuid::new_v4()));
        fs::write(&path, contents).expect("temp cert material should write");
        path
    }

    #[test]
    fn applies_read_consumer_defaults_without_tls_or_auth() {
        let mut config = ClientConfig::new();

        apply_kafka_read_consumer_config(&mut config, &sample_profile("none", "system-default"))
            .expect("plaintext consumer config should apply");

        assert_eq!(config.get("bootstrap.servers"), Some("localhost:9092"));
        assert_eq!(config.get("socket.timeout.ms"), Some("5000"));
        assert_eq!(config.get("session.timeout.ms"), Some("6000"));
        assert_eq!(config.get("enable.auto.commit"), Some("false"));
        assert_eq!(config.get("security.protocol"), None);
    }

    #[test]
    fn applies_tls_insecure_without_auth() {
        let mut config = ClientConfig::new();

        apply_kafka_security_config_with_secret(
            &mut config,
            &sample_profile("none", "tls-insecure"),
            None,
        )
        .expect("tls-insecure config should apply");

        assert_eq!(config.get("security.protocol"), Some("ssl"));
        assert_eq!(
            config.get("enable.ssl.certificate.verification"),
            Some("false")
        );
        assert_eq!(config.get("sasl.mechanism"), None);
    }

    #[test]
    fn applies_sasl_plain_over_tls() {
        let mut config = ClientConfig::new();
        let mut profile = sample_profile("sasl-plain", "tls-required");
        profile.auth_credential_ref = Some("kafka-admin".to_string());

        apply_kafka_security_config_with_secret(&mut config, &profile, Some("alice:secret"))
            .expect("sasl/tls config should apply");

        assert_eq!(config.get("security.protocol"), Some("sasl_ssl"));
        assert_eq!(config.get("sasl.mechanism"), Some("PLAIN"));
        assert_eq!(config.get("sasl.username"), Some("alice"));
        assert_eq!(config.get("sasl.password"), Some("secret"));
    }

    #[test]
    fn applies_mtls_paths_when_files_exist() {
        let ca_path = create_temp_file("helper-ca", "test-ca");
        let cert_path = create_temp_file("helper-cert", "test-cert");
        let key_path = create_temp_file("helper-key", "test-key");

        let mut profile = sample_profile("mtls", "tls-required");
        profile.tls_ca_cert_path = Some(ca_path.to_string_lossy().into_owned());
        profile.tls_client_cert_path = Some(cert_path.to_string_lossy().into_owned());
        profile.tls_client_key_path = Some(key_path.to_string_lossy().into_owned());

        let mut config = ClientConfig::new();
        apply_kafka_security_config_with_secret(&mut config, &profile, None)
            .expect("mTLS config should apply");

        assert_eq!(config.get("security.protocol"), Some("ssl"));
        assert_eq!(
            config.get("ssl.ca.location"),
            profile.tls_ca_cert_path.as_deref()
        );
        assert_eq!(
            config.get("ssl.certificate.location"),
            profile.tls_client_cert_path.as_deref()
        );
        assert_eq!(
            config.get("ssl.key.location"),
            profile.tls_client_key_path.as_deref()
        );

        let _ = fs::remove_file(ca_path);
        let _ = fs::remove_file(cert_path);
        let _ = fs::remove_file(key_path);
    }
}
