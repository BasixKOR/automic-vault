#![cfg(coverage)]
#![allow(dead_code)]

use std::sync::{Mutex, OnceLock};

fn global_test_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(Mutex::default)
}

macro_rules! radioisotope_source {
    ($path:literal) => {
        concat!(env!("AUTOMIC_VAULT_GENERATED_RADIOISOTOPES_REPO"), $path)
    };
}

mod docker_detect {
    include!(radioisotope_source!("/docker/detect.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::fs;

        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        #[test]
        fn covers_config_parsers_and_json_edges() {
            assert!(docker_config_contains_inline_secret(
                r#"{"auths":{"registry":{"identityToken":"token"}}}"#,
            ));
            assert!(!docker_config_contains_inline_secret(
                r#"{"auths":{"registry":{"auth":""}}}"#,
            ));
            assert!(docker_legacy_config_contains_secret(
                r#"{"registry":{"identitytoken":"token"}}"#,
            ));
            assert_eq!(
                credential_helper_values(r#"{"credHelpers":{"a":"av","b":"desktop"}}"#),
                vec!["av".to_string(), "desktop".to_string()]
            );
            assert!(config_has_default_helper(Some(
                r#"{"credsStore":"desktop"}"#
            )));
            assert!(!config_has_default_helper(Some(r#"{"credsStore":""}"#)));
            assert!(!config_has_default_helper(None));
            assert!(is_av_helper(" Automic-Vault-Docker "));
            assert!(!is_av_helper("desktop"));
            assert!(object_for_key(r#"{"auths":[]}"#, "auths").is_none());
            assert_eq!(
                object_value(r#"{"nested":{"quote":"a\"b"}} tail"#),
                Some(r#"{"nested":{"quote":"a\"b"}}"#)
            );
            assert!(object_value("not-object").is_none());
            assert_eq!(
                string_values_for_key(
                    r#"{"credsStore":"desktop","credsStore":"av"}"#,
                    "credsStore"
                ),
                vec!["desktop".to_string(), "av".to_string()]
            );
            assert_eq!(json_string_value(r#""a\"b" tail"#), Some(r#"a\"b"#));
            assert!(json_string_value("not-string").is_none());
        }

        #[cfg(unix)]
        #[test]
        fn covers_unix_group_socket_and_metadata_edges() {
            assert_eq!(
                group_file_line_name_and_gid(" docker :x:123: "),
                Some(("docker", 123))
            );
            assert!(group_file_line_name_and_gid("# comment").is_none());
            assert!(group_file_line_name_and_gid(":x:123:").is_none());
            assert!(group_file_line_name_and_gid("docker:x:not-a-gid:").is_none());
            assert!(group_file_contains_named_group_id(
                "docker:x:123:\n",
                "docker",
                &[123]
            ));
            assert!(!group_file_contains_named_group_id(
                "docker:x:123:\n",
                "docker",
                &[124]
            ));
            assert_eq!(docker_host_unix_socket_path("unix://"), None);
            assert_eq!(
                docker_host_unix_socket_path("unix:///tmp/docker.sock"),
                Some(PathBuf::from("/tmp/docker.sock"))
            );

            let _lock = crate::global_test_env_lock().lock().unwrap();
            let previous_host = std::env::var_os("DOCKER_HOST");
            unsafe {
                std::env::set_var("DOCKER_HOST", "unix:///tmp/docker-extra.sock");
            }
            assert!(docker_socket_paths().contains(&PathBuf::from("/tmp/docker-extra.sock")));
            unsafe {
                match previous_host {
                    Some(value) => std::env::set_var("DOCKER_HOST", value),
                    None => std::env::remove_var("DOCKER_HOST"),
                }
            }

            let path = std::env::temp_dir().join(format!("docker-metadata-{}", std::process::id()));
            fs::write(&path, "socket").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            let metadata = fs::metadata(&path).unwrap();
            assert!(metadata_is_writable_by_current_user(&metadata, &[]));
            fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
            let metadata = fs::metadata(&path).unwrap();
            assert!(!metadata_is_writable_by_current_user(&metadata, &[]));
            fs::remove_file(path).unwrap();
        }

        #[test]
        fn covers_env_paths_and_top_level_detection_edges() {
            let _lock = crate::global_test_env_lock().lock().unwrap();
            let root =
                std::env::temp_dir().join(format!("docker-detect-extra-{}", std::process::id()));
            let docker_config = root.join("custom-docker");
            let home = root.join("home");
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&docker_config).unwrap();
            fs::create_dir_all(&home).unwrap();
            fs::write(docker_config.join("config.json"), r#"{"credsStore":"av"}"#).unwrap();
            fs::write(home.join(".dockercfg"), r#"{"registry":{"auth":"secret"}}"#).unwrap();

            let previous_home = std::env::var_os("HOME");
            let previous_docker_config = std::env::var_os("DOCKER_CONFIG");
            unsafe {
                std::env::set_var("HOME", &home);
                std::env::set_var("DOCKER_CONFIG", &docker_config);
            }
            assert_eq!(
                docker_config_path().unwrap(),
                docker_config.join("config.json")
            );
            assert!(install_is_insecure().unwrap());
            assert!(
                install_insecurity_reasons()
                    .unwrap()
                    .iter()
                    .any(|reason| reason.contains("legacy config"))
            );
            assert!(
                read_to_string(&root)
                    .unwrap_err()
                    .contains("failed to read")
            );

            unsafe {
                std::env::remove_var("HOME");
                std::env::remove_var("DOCKER_CONFIG");
            }
            assert!(home_dir().unwrap_err().contains("HOME"));
            assert!(docker_config_path().unwrap_err().contains("HOME"));
            assert!(docker_desktop_is_installed().unwrap_err().contains("HOME"));

            unsafe {
                match previous_home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
                match previous_docker_config {
                    Some(value) => std::env::set_var("DOCKER_CONFIG", value),
                    None => std::env::remove_var("DOCKER_CONFIG"),
                }
            }
            fs::remove_dir_all(root).unwrap();
        }
    }
}

mod aws_cli_detect {
    include!(radioisotope_source!("/aws-cli/detect.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::fs;

        #[test]
        fn covers_credentials_and_login_cache_edges() {
            assert!(is_credentials_file_secret_key("aws_access_key_id"));
            assert!(!is_credentials_file_secret_key("region"));
            assert!(contains_json_string_value(
                r#"{"Credentials":{"AWS_SECRET_ACCESS_KEY":"secret"}}"#,
                "AWS_SECRET_ACCESS_KEY"
            ));
            assert!(!contains_json_string_value(
                r#"{"AWS_SECRET_ACCESS_KEY":""}"#,
                "AWS_SECRET_ACCESS_KEY"
            ));
            assert!(!contains_json_string_value(
                r#"{"AWS_SECRET_ACCESS_KEY""#,
                "AWS_SECRET_ACCESS_KEY"
            ));

            let root =
                std::env::temp_dir().join(format!("aws-detect-extra-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let credentials = root.join("credentials");
            fs::write(
                &credentials,
                "[profile dev]\naws_access_key_id = DEV\n[default]\nregion = us\n",
            )
            .unwrap();
            assert!(!credentials_file_is_insecure(&credentials).unwrap());
            fs::write(&credentials, "[default]\naws_access_key_id = AKIA\n").unwrap();
            assert!(credentials_file_is_insecure(&credentials).unwrap());
            assert!(
                credentials_file_is_insecure(&root)
                    .unwrap_err()
                    .contains("failed to read")
            );

            assert!(!login_cache_is_insecure(&root.join("missing")).unwrap());
            let not_dir = root.join("not-dir");
            fs::write(&not_dir, "").unwrap();
            assert!(
                login_cache_is_insecure(&not_dir)
                    .unwrap_err()
                    .contains("failed to read")
            );
            let cache = root.join("cache");
            fs::create_dir_all(&cache).unwrap();
            fs::write(cache.join("ignore.txt"), "secretAccessKey").unwrap();
            fs::write(cache.join("empty.json"), r#"{"secretAccessKey":""}"#).unwrap();
            assert!(!login_cache_is_insecure(&cache).unwrap());
            fs::write(cache.join("creds.json"), r#"{"accessKeyId":"AKIA"}"#).unwrap();
            assert!(login_cache_is_insecure(&cache).unwrap());
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn covers_top_level_env_selection() {
            let _lock = crate::global_test_env_lock().lock().unwrap();
            let root = std::env::temp_dir().join(format!("aws-detect-env-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let credentials = root.join("custom-credentials");
            fs::write(&credentials, "[default]\naws_secret_access_key = secret\n").unwrap();

            let previous_home = std::env::var_os("HOME");
            let previous_credentials = std::env::var_os("AWS_SHARED_CREDENTIALS_FILE");
            unsafe {
                std::env::remove_var("HOME");
                std::env::remove_var("AWS_SHARED_CREDENTIALS_FILE");
            }
            assert!(install_insecurity_reasons().unwrap_err().contains("HOME"));
            unsafe {
                std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", &credentials);
            }
            let reasons = install_insecurity_reasons().unwrap();
            assert_eq!(reasons.len(), 1);
            assert!(install_is_insecure().unwrap());

            unsafe {
                match previous_home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
                match previous_credentials {
                    Some(value) => std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", value),
                    None => std::env::remove_var("AWS_SHARED_CREDENTIALS_FILE"),
                }
            }
            fs::remove_dir_all(root).unwrap();
        }
    }
}

mod pulumi_detect {
    include!(radioisotope_source!("/pulumi/detect.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::fs;

        #[test]
        fn covers_path_selection_and_top_level_errors() {
            let _lock = crate::global_test_env_lock().lock().unwrap();
            let root =
                std::env::temp_dir().join(format!("pulumi-detect-extra-{}", std::process::id()));
            let credentials_dir = root.join("credentials-dir");
            let pulumi_home = root.join("pulumi-home");
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&credentials_dir).unwrap();
            fs::create_dir_all(&pulumi_home).unwrap();
            fs::write(
                credentials_dir.join("credentials.json"),
                r#"{"accessTokens":{"https://api.pulumi.com":"pul-secret"}}"#,
            )
            .unwrap();

            let previous_home = std::env::var_os("HOME");
            let previous_credentials_path = std::env::var_os("PULUMI_CREDENTIALS_PATH");
            let previous_pulumi_home = std::env::var_os("PULUMI_HOME");
            unsafe {
                std::env::set_var("PULUMI_CREDENTIALS_PATH", &credentials_dir);
                std::env::set_var("PULUMI_HOME", &pulumi_home);
                std::env::remove_var("HOME");
            }
            assert_eq!(
                pulumi_credentials_path().unwrap(),
                credentials_dir.join("credentials.json")
            );
            assert!(install_is_insecure().unwrap());
            unsafe {
                std::env::remove_var("PULUMI_CREDENTIALS_PATH");
            }
            assert_eq!(
                pulumi_credentials_path().unwrap(),
                pulumi_home.join("credentials.json")
            );
            unsafe {
                std::env::remove_var("PULUMI_HOME");
            }
            assert!(pulumi_credentials_path().unwrap_err().contains("HOME"));
            assert!(
                read_to_string(&root)
                    .unwrap_err()
                    .contains("failed to read")
            );

            unsafe {
                match previous_home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
                match previous_credentials_path {
                    Some(value) => std::env::set_var("PULUMI_CREDENTIALS_PATH", value),
                    None => std::env::remove_var("PULUMI_CREDENTIALS_PATH"),
                }
                match previous_pulumi_home {
                    Some(value) => std::env::set_var("PULUMI_HOME", value),
                    None => std::env::remove_var("PULUMI_HOME"),
                }
            }
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn covers_json_parser_edges() {
            assert!(!pulumi_credentials_contains_access_token(
                r#"{"accessTokens""#
            ));
            assert!(!pulumi_credentials_contains_access_token(
                r#"{"accessTokens":[]}"#
            ));
            assert!(matching_object_end(r#"{"nested":"a\"b"} tail"#).is_some());
            assert!(matching_object_end(r#"{"unterminated":true"#).is_none());
            assert!(!object_contains_non_empty_string_value("not-json"));
            assert!(!object_contains_non_empty_string_value(r#""key" "value""#));
            assert!(!object_contains_non_empty_string_value(r#""key": true"#));
            assert!(object_contains_non_empty_string_value(
                r#""empty":"", "token":"secret""#
            ));
            assert_eq!(skip_json_space_and_commas(" ,\n\tkey", 0), 4);
            assert_eq!(skip_json_space(" \n\tkey", 0), 3);
            assert_eq!(
                parse_json_string(r#""a\"b" tail"#, 0),
                Some((r#"a"b"#.to_string(), 6))
            );
            assert!(parse_json_string("unterminated", 0).is_none());
        }
    }
}
