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

mod snyk_migrate {
    include!(radioisotope_source!("/snyk/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;

        #[test]
        fn covers_secret_detection_and_assignment_errors() {
            assert_eq!(keys(), &["SNYK_ENV_ASSIGNMENTS"]);
            assert!(config_has_secrets(r#"{"clientSecret":"secret"}"#));
            assert!(config_has_secrets(r#"{"api":"secret"}"#));
            assert!(!config_has_secrets(r#"{"api":""}"#));
            assert!(!json_string_key_has_nonempty_value(
                r#"{"api" "missing-colon"}"#,
                "api"
            ));
            assert!(!json_string_key_has_nonempty_value(r#"{"api": 12}"#, "api"));

            assert!(
                snyk_env_assignments(r#"{"oci-registry-password":"secret"}"#)
                    .unwrap_err()
                    .contains("registry passwords")
            );
            assert!(
                snyk_env_assignments(r#"{"api":"one","token":"two"}"#)
                    .unwrap_err()
                    .contains("conflicting")
            );
            assert!(
                snyk_env_assignments("{\"api\":\"line\\nbreak\"}")
                    .unwrap_err()
                    .contains("SNYK_TOKEN")
            );

            let sanitized = sanitized_config_json(r#"{"api":"one","oauthToken":"two"}"#).unwrap();
            assert!(sanitized.contains("\"api\": \"\""));
            assert!(sanitized.contains("\"oauthToken\": \"\""));
        }
    }
}

mod algolia_migrate {
    include!(radioisotope_source!("/algolia/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;

        #[test]
        fn covers_profile_parser_and_env_errors() {
            assert_eq!(keys(), &["ALGOLIA_ENV_ASSIGNMENTS"]);
            assert!(config_contains_secret("api_key = 'secret'"));
            assert!(!config_contains_secret("api_key = ''"));
            assert!(!toml_string_field_is_present(
                "not an assignment",
                "api_key"
            ));
            assert_eq!(toml_string_value(r#""a\"b""#).unwrap(), "a\"b");
            assert!(toml_string_value("bare").is_none());
            assert!(toml_string_value("\"unterminated").is_none());

            assert!(
                algolia_env_assignments("[one]\napi_key='a'\n[two]\napi_key='b'\n")
                    .unwrap_err()
                    .contains("multiple profiles")
            );
            assert!(
                algolia_env_assignments("[default]\napi_key='a'\n")
                    .unwrap_err()
                    .contains("application_id")
            );
            assert!(
                algolia_env_assignments("[default]\ncrawler_api_key='a'\n")
                    .unwrap_err()
                    .contains("crawler_user_id")
            );
            assert!(reject_env_line_breaks("ALGOLIA_API_KEY", "a\nb").is_err());

            let sanitized =
                sanitized_config_toml("[default]\napi_key = 'secret' # keep\nunknown = 'x'\n");
            assert!(sanitized.contains("api_key = \"\""));
            assert!(sanitized.contains("unknown = 'x'"));
        }
    }
}

mod akamai_migrate {
    include!(radioisotope_source!("/akamai/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;

        #[test]
        fn covers_section_and_assignment_edges() {
            assert_eq!(keys(), &["AKAMAI_ENV_ASSIGNMENTS"]);
            assert!(config_has_edgegrid_secrets("client_token = 'token'"));
            assert!(!config_has_edgegrid_secrets("client_token = ''"));
            assert_eq!(unquote_ini_value("'quoted'"), "quoted");
            assert_eq!(unquote_ini_value("plain"), "plain");
            assert_eq!(env_section_prefix("default").unwrap(), "");
            assert_eq!(env_section_prefix("prod_1").unwrap(), "PROD_1_");
            assert!(env_section_prefix("bad-name").is_err());

            let mut assignments = Vec::new();
            push_assignment(
                &mut assignments,
                "AKAMAI_HOST".to_string(),
                "one".to_string(),
            )
            .unwrap();
            push_assignment(
                &mut assignments,
                "AKAMAI_HOST".to_string(),
                "one".to_string(),
            )
            .unwrap();
            assert!(
                push_assignment(
                    &mut assignments,
                    "AKAMAI_HOST".to_string(),
                    "two".to_string()
                )
                .unwrap_err()
                .contains("conflicting")
            );
            assert!(
                push_assignment(
                    &mut assignments,
                    "AKAMAI_TOKEN".to_string(),
                    "a\nb".to_string()
                )
                .unwrap_err()
                .contains("line breaks")
            );

            let missing = edgerc_migration(
                "[default]\nclient_token = token\nclient_secret = secret\naccess_token = access\n",
            )
            .unwrap_err();
            assert!(missing.contains("host"));
            let unsafe_section = edgerc_migration(
                "[bad-name]\nhost = h\nclient_token = t\nclient_secret = s\naccess_token = a\n",
            )
            .unwrap_err();
            assert!(unsafe_section.contains("safe environment variable"));
        }
    }
}

mod twine_migrate {
    include!(radioisotope_source!("/twine/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;

        #[test]
        fn covers_repository_userinfo_and_conflicts() {
            assert_eq!(keys(), &["TWINE_ENV_ASSIGNMENTS"]);
            assert_eq!(sanitize_line("# comment"), "# comment");
            assert_eq!(
                sanitize_line("repository https://example.test"),
                "repository https://example.test"
            );
            assert_eq!(
                strip_url_userinfo("https://user:pass@example.test/simple"),
                "https://example.test/simple"
            );
            assert_eq!(
                strip_url_userinfo("https://example.test/simple"),
                "https://example.test/simple"
            );
            assert_eq!(
                repository_userinfo("https://user@example.test/simple")
                    .unwrap()
                    .username
                    .as_deref(),
                Some("user")
            );
            assert!(repository_userinfo("https://example.test/path@later").is_none());

            assert!(twine_env_assignments("[one]\nusername=a\npassword=b\nrepository=https://one.test\n[two]\nusername=a\npassword=b\nrepository=https://two.test\n")
                .unwrap_err()
                .contains("multiple repositories"));
            assert!(
                twine_env_assignments("[private]\npassword=b\nrepository=https://private.test\n")
                    .unwrap_err()
                    .contains("without a username")
            );
            assert!(twine_env_assignments("[pypi]\nusername=a\nrepository=https://a:other@upload.pypi.org/legacy/\npassword=b\n")
                .unwrap_err()
                .contains("conflicting password"));
            assert!(reject_env_line_breaks("TWINE_PASSWORD", "a\nb").is_err());
        }
    }
}

mod luarocks_migrate {
    include!(radioisotope_source!("/luarocks/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::path::{Path, PathBuf};

        #[test]
        fn covers_assignment_parser_edges() {
            assert_eq!(keys(), &["LUAROCKS_API_KEY"]);
            assert!(parse_key_assignment("-- key = 'secret'").is_none());
            assert!(parse_key_assignment("not_key = 'secret'").is_none());
            assert!(parse_key_assignment("key = nil").is_none());
            assert!(parse_key_assignment("key = ''").is_none());
            assert_eq!(
                parse_key_assignment("upload.key = \"sec\\\"ret\"")
                    .unwrap()
                    .value,
                "sec\\\"ret"
            );
            assert!(key_side_names_key("upload['key']"));
            assert!(!key_side_names_key("monkey"));
            assert_eq!(
                upload_config_path_for_user_config(Path::new("luarocks.lua")),
                PathBuf::from("upload_config.lua")
            );

            assert!(
                upload_config_migration("key = 'one'\nkey = 'two'\n")
                    .unwrap_err()
                    .contains("multiple distinct")
            );
            assert!(upload_config_migration("key = 'one'\n").unwrap().is_some());
            assert!(upload_config_migration("return {}\n").unwrap().is_none());
        }
    }
}

mod midnight_commander_migrate {
    include!(radioisotope_source!("/midnight-commander/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;

        #[test]
        fn covers_profile_secret_detection_edges() {
            assert_eq!(keys(), &["MC_INI", "MC_HOTLIST", "MC_PANELS_INI"]);
            assert!(profile_has_secrets("ftpfs_password = secret\n"));
            assert!(!profile_has_secrets("ftpfs_password = <hidden>\n"));
            assert!(contains_url_password("ftp://user:pass@example.test/path"));
            assert!(contains_url_password("sftp:user:pass@example.test/path"));
            assert!(!contains_url_password("ftp://user:@example.test/path"));
            assert!(!contains_url_password("plain text"));
            assert!(line_has_password_setting(" password = secret "));
            assert!(!line_has_password_setting(" password =  "));
        }
    }
}

mod snowflake_cli_migrate {
    include!(radioisotope_source!("/snowflake-cli/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::cell::RefCell;
        use std::fs;
        use std::path::PathBuf;

        #[derive(Default)]
        struct Store {
            values: RefCell<Vec<(String, String)>>,
            fail: bool,
        }

        impl CredentialStore for Store {
            fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
                if self.fail {
                    return Err("store failed".to_string());
                }
                self.values
                    .borrow_mut()
                    .push((key.to_string(), value.to_string()));
                Ok(())
            }
        }

        #[test]
        fn covers_line_parsing_bundle_and_storage_edges() {
            assert_eq!(keys(), &["SNOWFLAKE_ENV_ASSIGNMENTS"]);
            assert!(
                keychain_store_secret("service", "account", "value")
                    .unwrap_err()
                    .contains("keychain")
            );
            assert_eq!(section_name(" [ default ] "), Some("default"));
            assert!(toml_value_is_nonempty("'secret'"));
            assert!(!toml_value_is_nonempty("''"));
            assert_eq!(toml_string_value(r#""a\"b""#).as_deref(), Some("a\"b"));
            assert_eq!(toml_string_value("'abc'").as_deref(), Some("abc"));
            assert!(toml_string_value("\"unterminated").is_none());
            assert_eq!(env_connection_suffix("prod_1").unwrap(), "PROD_1");
            assert!(env_connection_suffix("").is_err());
            assert!(env_connection_suffix("prod-west").is_err());
            assert!(reject_env_line_breaks("a\rb").is_err());

            let no_change = file_migration(
                "[connections.default]\nuser = 'me'\n",
                ConfigFileKind::ConfigToml,
            )
            .unwrap();
            assert!(!no_change.changed);
            assert_eq!(no_change.sanitized, "[connections.default]\nuser = 'me'\n");

            let outside =
                file_migration("password = 'secret'\n", ConfigFileKind::ConfigToml).unwrap_err();
            assert!(outside.contains("outside a connection"));
            assert!(
                file_migration(
                    "[connections.default]\nprivate_key_file_pwd = ''\n",
                    ConfigFileKind::ConfigToml,
                )
                .unwrap()
                .assignments
                .is_empty()
            );

            let dedup = file_migration(
                "[connections.default]\npassword = 'secret'\npassword = 'secret'\n",
                ConfigFileKind::ConfigToml,
            )
            .unwrap();
            assert_eq!(dedup.assignments.len(), 1);

            let bundle = ConfigBundle {
                dir: PathBuf::from("snowflake"),
                config: Some(FileState {
                    path: PathBuf::from("config.toml"),
                    sanitized: String::new(),
                    changed: false,
                    assignments: vec!["A=1".to_string()],
                }),
                connections: Some(FileState {
                    path: PathBuf::from("connections.toml"),
                    sanitized: String::new(),
                    changed: false,
                    assignments: vec!["A=1".to_string(), "B=2".to_string()],
                }),
            };
            assert!(!bundle.has_sensitive_values());
            assert_eq!(
                bundle.assignments(),
                vec!["A=1".to_string(), "B=2".to_string()]
            );

            let empty_assignment_bundle = ConfigBundle {
                dir: PathBuf::from("snowflake"),
                config: Some(FileState {
                    path: PathBuf::from("config.toml"),
                    sanitized: String::new(),
                    changed: true,
                    assignments: Vec::new(),
                }),
                connections: None,
            };
            assert!(!migrate_bundle(empty_assignment_bundle, &Store::default()).unwrap());

            let write_dir =
                std::env::temp_dir().join(format!("snowflake-write-dir-{}", std::process::id()));
            let _ = fs::remove_dir_all(&write_dir);
            fs::create_dir_all(&write_dir).unwrap();
            let write_error_bundle = ConfigBundle {
                dir: write_dir.clone(),
                config: Some(FileState {
                    path: write_dir.clone(),
                    sanitized: String::new(),
                    changed: true,
                    assignments: vec!["A=1".to_string()],
                }),
                connections: None,
            };
            assert!(
                migrate_bundle(write_error_bundle, &Store::default())
                    .unwrap_err()
                    .contains("failed to write")
            );
            fs::remove_dir_all(write_dir).unwrap();

            let store_error_bundle = ConfigBundle {
                dir: PathBuf::from("snowflake"),
                config: Some(FileState {
                    path: PathBuf::from("config.toml"),
                    sanitized: String::new(),
                    changed: false,
                    assignments: vec!["A=1".to_string()],
                }),
                connections: None,
            };
            assert!(
                migrate_bundle(
                    store_error_bundle,
                    &Store {
                        values: RefCell::new(Vec::new()),
                        fail: true,
                    },
                )
                .unwrap_err()
                .contains("store failed")
            );
        }

        #[test]
        fn covers_default_directory_selection_and_multi_match_error() {
            let _lock = crate::global_test_env_lock().lock().unwrap();
            let previous_home = std::env::var_os("HOME");
            unsafe {
                std::env::remove_var("HOME");
            }
            assert!(candidate_directories().unwrap_err().contains("HOME"));

            let home = std::env::temp_dir().join(format!("snowflake-home-{}", std::process::id()));
            let _ = fs::remove_dir_all(&home);
            fs::create_dir_all(home.join(".snowflake")).unwrap();
            fs::create_dir_all(home.join(".config/snowflake")).unwrap();
            fs::write(
                home.join(".snowflake/config.toml"),
                "[connections.default]\npassword = 'one'\n",
            )
            .unwrap();
            fs::write(
                home.join(".config/snowflake/connections.toml"),
                "[prod]\npassword = 'two'\n",
            )
            .unwrap();
            unsafe {
                std::env::set_var("HOME", &home);
            }
            assert_eq!(candidate_directories().unwrap().len(), 3);
            assert!(
                migrate_default_configs(&Store::default())
                    .unwrap_err()
                    .contains("multiple Snowflake")
            );
            unsafe {
                match previous_home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
            }
            fs::remove_dir_all(home).unwrap();
        }
    }
}

mod grafanactl_migrate {
    include!(radioisotope_source!("/grafanactl/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::cell::RefCell;
        use std::fs;

        #[derive(Default)]
        struct Store {
            values: RefCell<Vec<(String, String)>>,
            fail: bool,
        }

        impl CredentialStore for Store {
            fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
                if self.fail {
                    return Err("store failed".to_string());
                }
                self.values
                    .borrow_mut()
                    .push((key.to_string(), value.to_string()));
                Ok(())
            }
        }

        #[test]
        fn covers_path_detection_secret_parsing_and_env_edges() {
            assert_eq!(keys(), &["GRAFANACTL_ENV_ASSIGNMENTS"]);
            assert!(
                keychain_store_secret("service", "account", "value")
                    .unwrap_err()
                    .contains("keychain")
            );
            assert!(config_contains_secret("token: 'secret' # comment"));
            assert!(yaml_secret_line_is_present("password: \"secret\""));
            assert!(!yaml_secret_line_is_present("password:"));
            assert!(!yaml_secret_line_is_present("not yaml"));
            assert_eq!(unquote_yaml_scalar("'quoted'"), "quoted");
            assert_eq!(unquote_yaml_scalar("plain"), "plain");
            assert!(reject_env_line_breaks("GRAFANA_TOKEN", "a\nb").is_err());

            let token_linebreak = GrafanaContext {
                name: "default".to_string(),
                token: Some("a\rb".to_string()),
                user: None,
                password: None,
            };
            assert!(
                token_linebreak
                    .env_assignments()
                    .unwrap_err()
                    .contains("line breaks")
            );
            let user_linebreak = GrafanaContext {
                name: "default".to_string(),
                token: None,
                user: Some("a\nb".to_string()),
                password: Some("secret".to_string()),
            };
            assert!(
                user_linebreak
                    .env_assignments()
                    .unwrap_err()
                    .contains("GRAFANA_USER")
            );
            let password_linebreak = GrafanaContext {
                name: "default".to_string(),
                token: None,
                user: Some("admin".to_string()),
                password: Some("a\rb".to_string()),
            };
            assert!(
                password_linebreak
                    .env_assignments()
                    .unwrap_err()
                    .contains("GRAFANA_PASSWORD")
            );

            let contexts = grafana_secret_contexts(
                "outside: true\ncontexts:\n  default:\n    grafana:\n      token: ''\n      user: admin\n      password: secret\n",
            );
            assert_eq!(contexts.len(), 1);
            assert_eq!(contexts[0].name, "default");

            assert_eq!(
                sanitized_config_yaml("contexts:\n  default:\n    grafana:\n      token: secret"),
                "contexts:\n  default:\n    grafana:\n      token: \"\""
            );
        }

        #[test]
        fn covers_config_paths_and_file_errors() {
            let _lock = crate::global_test_env_lock().lock().unwrap();
            let previous_home = std::env::var_os("HOME");
            let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
            let root = std::env::temp_dir().join(format!("grafanactl-home-{}", std::process::id()));
            let xdg = root.join("xdg");
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&xdg).unwrap();
            unsafe {
                std::env::set_var("XDG_CONFIG_HOME", &xdg);
                std::env::remove_var("HOME");
            }
            assert_eq!(
                grafanactl_config_path().unwrap(),
                xdg.join("grafanactl/config.yaml")
            );
            unsafe {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            assert!(grafanactl_config_path().unwrap_err().contains("HOME"));

            let path = root.join("config.yaml");
            fs::write(
                &path,
                "contexts:\n  default:\n    grafana:\n      token: secret\n",
            )
            .unwrap();
            assert!(
                migrate_config_file(
                    &path,
                    &Store {
                        values: RefCell::new(Vec::new()),
                        fail: true,
                    },
                )
                .unwrap_err()
                .contains("store failed")
            );
            assert!(
                migrate_config_file(&root, &Store::default())
                    .unwrap_err()
                    .contains("failed to read")
            );

            unsafe {
                match previous_home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
                match previous_xdg {
                    Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                    None => std::env::remove_var("XDG_CONFIG_HOME"),
                }
            }
            fs::remove_dir_all(root).unwrap();
        }
    }
}

mod nuget_migrate {
    include!(radioisotope_source!("/nuget/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::cell::RefCell;
        use std::fs;

        struct ErrorStore;

        impl CredentialStore for ErrorStore {
            fn store_secret(&self, _key: &str, _value: &str) -> Result<(), String> {
                Err("store failed".to_string())
            }
        }

        #[derive(Default)]
        struct Store {
            values: RefCell<Vec<(String, String)>>,
        }

        impl CredentialStore for Store {
            fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
                self.values
                    .borrow_mut()
                    .push((key.to_string(), value.to_string()));
                Ok(())
            }
        }

        #[test]
        fn covers_xml_helpers_and_secret_detectors() {
            assert_eq!(
                keys(),
                &[
                    "NUGET_MONO_CONFIG_XML",
                    "NUGET_DOTNET_CONFIG_XML",
                    "NUGET_PACKAGE_SOURCE_CREDENTIALS_JSON"
                ]
            );
            assert!(
                keychain_store_secret("service", "account", "value")
                    .unwrap_err()
                    .contains("keychain")
            );
            assert!(config_has_secrets(
                r#"<configuration><config><add key="http_proxy.password" value="secret" /></config></configuration>"#,
            ));
            assert!(config_has_secrets(
                r#"<configuration><clientCertificates><certificate password="secret" /></clientCertificates></configuration>"#,
            ));
            assert!(!has_configured_api_key(
                r#"<apikeys><add key="x" value="" /></apikeys>"#
            ));
            assert!(package_source_credentials("<configuration />").is_empty());
            assert!(xml_section("<configuration />", "missing").is_none());
            assert!(
                xml_section_range("<apikeys><add /></apikeys>", "packageSourceCredentials")
                    .is_none()
            );
            assert!(xml_section_body_range("<apikeys><add />", "apikeys").is_none());
            assert!(add_tags("<add key=\"x\"").is_empty());
            assert!(xml_attr(r#"<add key=value value='secret' />"#, "value").is_none());
            assert_eq!(
                xml_attr(r#"<add value='secret' />"#, "value"),
                Some("secret".to_string())
            );
            assert_eq!(
                decode_xml_element_name("private_x0020_feed"),
                "private feed"
            );
            assert_eq!(decode_xml_element_name("bad_xzzzz_tail"), "bad_xzzzz_tail");
            assert_eq!(xml_unescape("&lt;&gt;&amp;&quot;&apos;"), "<>&\"'");
            assert_eq!(
                sanitize_package_source_credentials(
                    "<configuration><apikeys><add key=\"x\" value=\"s\" /></apikeys></configuration>"
                ),
                "<configuration />\n"
            );
            assert_eq!(
                sanitize_config_for_storage("<configuration></configuration>"),
                "<configuration></configuration>\n"
            );

            let credentials = vec![SourceCredential {
                name: "private".to_string(),
                uri: None,
                username: "user".to_string(),
                password: "pass".to_string(),
            }];
            assert!(
                source_credentials_json(&credentials)
                    .unwrap()
                    .contains("private")
            );
        }

        #[test]
        fn covers_env_paths_and_migration_error_edges() {
            let _lock = crate::global_test_env_lock().lock().unwrap();
            let previous_home = std::env::var_os("HOME");
            let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
            unsafe {
                std::env::remove_var("HOME");
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            assert!(user_home().unwrap_err().contains("HOME"));

            let root = std::env::temp_dir().join(format!("nuget-home-{}", std::process::id()));
            let xdg = root.join("xdg");
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&xdg).unwrap();
            unsafe {
                std::env::set_var("HOME", &root);
                std::env::set_var("XDG_CONFIG_HOME", &xdg);
            }
            let configs = nuget_configs().unwrap();
            assert_eq!(configs[0].path, xdg.join("NuGet/NuGet.Config"));

            let config = root.join("NuGet.Config");
            fs::write(
                &config,
                r#"<configuration><packageSourceCredentials><private><add key="Username" value="u" /><add key="Password" value="p" /></private></packageSourceCredentials></configuration>"#,
            )
            .unwrap();
            let configs = vec![NuGetConfig {
                path: config.clone(),
                env_key: NUGET_MONO_CONFIG_ENV_KEY,
            }];
            assert!(
                migrate_credentials_files(&configs, &ErrorStore)
                    .unwrap_err()
                    .contains("store failed")
            );

            let missing = root.join("missing.config");
            assert!(
                !migrate_credentials_files(
                    &[NuGetConfig {
                        path: missing,
                        env_key: NUGET_MONO_CONFIG_ENV_KEY,
                    }],
                    &Store::default(),
                )
                .unwrap()
            );
            assert!(
                migrate_credentials_files(
                    &[NuGetConfig {
                        path: root.clone(),
                        env_key: NUGET_MONO_CONFIG_ENV_KEY,
                    }],
                    &Store::default(),
                )
                .unwrap_err()
                .contains("failed to read")
            );

            unsafe {
                match previous_home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
                match previous_xdg {
                    Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                    None => std::env::remove_var("XDG_CONFIG_HOME"),
                }
            }
            fs::remove_dir_all(root).unwrap();
        }
    }
}

mod aws_cli_migrate {
    include!(radioisotope_source!("/aws-cli/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::cell::RefCell;
        use std::fs;

        struct Store {
            values: RefCell<Vec<(String, String)>>,
            fail_on_secret: bool,
        }

        impl Default for Store {
            fn default() -> Self {
                Self {
                    values: RefCell::new(Vec::new()),
                    fail_on_secret: false,
                }
            }
        }

        impl CredentialStore for Store {
            fn store_secret(&self, key: &str, value: &str) -> Result<(), String> {
                if self.fail_on_secret && key == AWS_SECRET_ACCESS_KEY_ENV_KEY {
                    return Err("secret store failed".to_string());
                }
                self.values
                    .borrow_mut()
                    .push((key.to_string(), value.to_string()));
                Ok(())
            }
        }

        #[test]
        fn covers_ini_json_and_config_helpers() {
            assert!(
                keychain_store_secret("service", "account", "value")
                    .unwrap_err()
                    .contains("keychain")
            );
            assert!(
                default_credentials("[default\nkey=value\n", Path::new("aws"))
                    .unwrap_err()
                    .contains("invalid section")
            );
            assert!(
                default_credentials("[]\nkey=value\n", Path::new("aws"))
                    .unwrap_err()
                    .contains("empty section")
            );
            assert!(
                default_credentials("key=value\n", Path::new("aws"))
                    .unwrap_err()
                    .contains("before any section")
            );
            assert!(
                default_credentials("[default]\naws_access_key_id = AKIA\n", Path::new("aws"))
                    .unwrap_err()
                    .contains("missing aws_secret")
            );
            assert!(
                default_credentials(
                    "[default]\naws_secret_access_key = secret\n",
                    Path::new("aws")
                )
                .unwrap_err()
                .contains("missing aws_access")
            );
            assert_eq!(
                split_ini_assignment(" key = value "),
                Some(("key", "value"))
            );
            assert_eq!(parse_section_header("[ default ]"), Some("default"));
            assert!(parse_section_header("[]").is_none());
            assert!(is_plaintext_aws_key("aws_access_key_id"));
            assert!(!is_plaintext_aws_key("region"));
            assert!(login_cache_file_has_credentials(
                r#"{"AWS_ACCESS_KEY_ID":"AKIA"}"#
            ));
            assert!(!contains_json_string_value(
                r#"{"AWS_ACCESS_KEY_ID":""}"#,
                "AWS_ACCESS_KEY_ID"
            ));
            assert!(!contains_json_string_value(
                r#"{"AWS_ACCESS_KEY_ID""#,
                "AWS_ACCESS_KEY_ID"
            ));

            assert_eq!(
                remove_default_plaintext_key_lines(
                    "[default]\naws_access_key_id = AKIA\nregion = us\n[dev]\naws_secret_access_key = keep\n"
                ),
                "[default]\nregion = us\n[dev]\naws_secret_access_key = keep\n"
            );
            assert_eq!(
                upsert_default_credential_process("[default]\n\nregion = us\n"),
                "[default]\n\nregion = us\ncredential_process = /usr/local/bin/av credential-helper aws\n"
            );
            assert_eq!(
                upsert_default_credential_process(
                    "[default]\ncredential_process = /usr/local/bin/av credential-helper aws\n"
                ),
                "[default]\ncredential_process = /usr/local/bin/av credential-helper aws\n"
            );
        }

        #[test]
        fn covers_store_home_login_cache_and_file_errors() {
            let _lock = crate::global_test_env_lock().lock().unwrap();
            let previous_home = std::env::var_os("HOME");
            unsafe {
                std::env::remove_var("HOME");
            }
            assert!(home_path().unwrap_err().contains("HOME"));

            let credentials = AwsCredentials {
                access_key_id: "AKIA".to_string(),
                secret_access_key: "secret".to_string(),
            };
            assert!(
                store_credentials(
                    &Store {
                        values: RefCell::new(Vec::new()),
                        fail_on_secret: true,
                    },
                    &credentials,
                )
                .unwrap_err()
                .contains("secret store failed")
            );

            let root = std::env::temp_dir().join(format!("aws-extra-{}", std::process::id()));
            let cache = root.join(AWS_LOGIN_CACHE_PATH);
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&cache).unwrap();
            fs::write(cache.join("ignore.txt"), "not json").unwrap();
            fs::write(cache.join("empty.json"), r#"{"accessKeyId":""}"#).unwrap();
            assert!(!warn_about_login_cache(&cache).unwrap());
            fs::write(cache.join("creds.json"), r#"{"secretAccessKey":"secret"}"#).unwrap();
            assert!(!warn_about_login_cache(&cache).unwrap());
            assert!(!warn_about_login_cache(&root.join("missing-cache")).unwrap());
            fs::write(root.join("not-dir"), "").unwrap();
            assert!(
                warn_about_login_cache(&root.join("not-dir"))
                    .unwrap_err()
                    .contains("failed to read")
            );

            let config = root.join("config");
            fs::write(
                &config,
                "[default]\ncredential_process = /usr/local/bin/av credential-helper aws\n",
            )
            .unwrap();
            ensure_credential_process_config(&config).unwrap();
            assert!(
                ensure_credential_process_config(&root)
                    .unwrap_err()
                    .contains("failed to read")
            );

            unsafe {
                match previous_home {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
            }
            fs::remove_dir_all(root).unwrap();
        }
    }
}

mod openstack_migrate {
    include!(radioisotope_source!("/openstackclient/migrate.rs"));

    #[cfg(test)]
    mod av_extra_tests {
        use super::*;
        use std::path::PathBuf;

        fn state(original: &str) -> FileState {
            FileState {
                path: PathBuf::from("clouds.yaml"),
                original: original.to_string(),
                sanitized: sanitized_config(original),
                changed: sanitized_config(original) != original,
            }
        }

        #[test]
        fn covers_yaml_parser_and_env_assignment_edges() {
            assert_eq!(
                keys(),
                &[
                    "OPENSTACK_ENV_ASSIGNMENTS",
                    "OPENSTACK_CLOUDS_YAML",
                    "OPENSTACK_SECURE_YAML"
                ]
            );
            assert_eq!(
                sanitized_config("clouds:\n  prod:\n    region_name: us\n"),
                "clouds:\n  prod:\n    region_name: us\n"
            );
            let mut changed = false;
            assert_eq!(
                sanitize_line("  - password: secret", &mut changed),
                "  - password: \"\""
            );
            assert!(changed);
            assert_eq!(trim_yaml_list_marker("- token: secret"), "token: secret");

            assert!(
                parse_openstack_config("not-yaml\nclouds:\n  prod:\n    token: t\n")
                    .unwrap()
                    .secrets
                    .len()
                    == 1
            );
            assert!(
                simple_yaml_scalar("|\n  multiline")
                    .unwrap_err()
                    .contains("multiline")
            );
            assert!(
                simple_yaml_scalar("{nested: value}")
                    .unwrap_err()
                    .contains("structured")
            );
            assert_eq!(
                simple_yaml_scalar("'quoted'").unwrap().as_deref(),
                Some("quoted")
            );
            assert_eq!(
                simple_yaml_scalar("\"quoted\"").unwrap().as_deref(),
                Some("quoted")
            );
            assert_eq!(simple_yaml_scalar("\"\"").unwrap(), None);

            let multi_cloud = state("clouds:\n  one:\n    token: t\n  two:\n    token: t\n");
            assert!(env_migration(Some(&multi_cloud), None).unwrap().is_none());
            let line_break = state("clouds:\n  prod:\n    token: \"a\rb\"\n");
            assert!(
                env_migration(Some(&line_break), None)
                    .unwrap_err()
                    .contains("line breaks")
            );
        }
    }
}
