ALTER TABLE cluster_profiles ADD COLUMN auth_credential_ref TEXT NULL;
ALTER TABLE cluster_profiles ADD COLUMN tls_ca_cert_path TEXT NULL;
ALTER TABLE cluster_profiles ADD COLUMN tls_client_cert_path TEXT NULL;
ALTER TABLE cluster_profiles ADD COLUMN tls_client_key_path TEXT NULL;
