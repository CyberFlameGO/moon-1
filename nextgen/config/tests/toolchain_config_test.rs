mod utils;

use moon_config::{BinConfig, BinEntry, NodePackageManager, ToolchainConfig};
use proto_core::{Id, PluginLocator, ToolsConfig, UnresolvedVersionSpec};
use starbase_sandbox::create_sandbox;
use std::env;
use utils::*;

const FILENAME: &str = ".moon/toolchain.yml";

mod toolchain_config {
    use super::*;

    #[test]
    #[should_panic(
        expected = "unknown field `unknown`, expected one of `$schema`, `extends`, `deno`, `node`, `rust`, `typescript`"
    )]
    fn error_unknown_field() {
        test_load_config(FILENAME, "unknown: 123", |path| {
            ToolchainConfig::load_from(path, &ToolsConfig::default())
        });
    }

    #[test]
    fn loads_defaults() {
        let config = test_load_config(FILENAME, "{}", |path| {
            ToolchainConfig::load_from(path, &ToolsConfig::default())
        });

        assert!(config.deno.is_none());
        assert!(config.node.is_none());
        assert!(config.rust.is_none());
        assert!(config.typescript.is_none());
    }

    mod extends {
        use super::*;

        #[test]
        fn recursive_merges() {
            let sandbox = create_sandbox("extends/toolchain");
            let config = test_config(sandbox.path().join("base-2.yml"), |path| {
                ToolchainConfig::load(sandbox.path(), path, &ToolsConfig::default())
            });

            let node = config.node.unwrap();

            assert_eq!(node.version.unwrap(), "4.5.6");
            assert!(node.add_engines_constraint);
            assert!(!node.dedupe_on_lockfile_change);
            assert_eq!(node.package_manager, NodePackageManager::Yarn);

            let yarn = node.yarn.unwrap();

            assert_eq!(yarn.version.unwrap(), "3.3.0");
        }

        #[test]
        fn recursive_merges_typescript() {
            let sandbox = create_sandbox("extends/toolchain");
            let config = test_config(sandbox.path().join("typescript-2.yml"), |path| {
                ToolchainConfig::load(sandbox.path(), path, &ToolsConfig::default())
            });

            let typescript = config.typescript.unwrap();

            assert_eq!(typescript.root_config_file_name, "tsconfig.root.json");
            assert!(!typescript.create_missing_config);
            assert!(typescript.sync_project_references);
        }
    }

    mod deno {
        use super::*;

        #[test]
        fn uses_defaults() {
            let config = test_load_config(FILENAME, "deno: {}", |path| {
                ToolchainConfig::load_from(path, &ToolsConfig::default())
            });

            let cfg = config.deno.unwrap();

            assert_eq!(cfg.deps_file, "deps.ts".to_owned());
            assert!(!cfg.lockfile);
        }

        #[test]
        fn sets_values() {
            let config = test_load_config(
                FILENAME,
                r"
deno:
  depsFile: dependencies.ts
  lockfile: true
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.deno.unwrap();

            assert_eq!(cfg.deps_file, "dependencies.ts".to_owned());
            assert!(cfg.lockfile);
        }

        #[test]
        fn can_set_bin_objects() {
            let config = test_load_config(
                FILENAME,
                r"
deno:
  bins:
    - https://deno.land/std@0.192.0/http/file_server.ts
    - bin: https://deno.land/std@0.192.0/http/file_server.ts
      name: 'fs'
      force: true
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.deno.unwrap();

            assert_eq!(
                cfg.bins,
                vec![
                    BinEntry::Name("https://deno.land/std@0.192.0/http/file_server.ts".into()),
                    BinEntry::Config(BinConfig {
                        bin: "https://deno.land/std@0.192.0/http/file_server.ts".into(),
                        name: Some("fs".into()),
                        force: true,
                        ..BinConfig::default()
                    }),
                ]
            );
        }

        #[test]
        fn enables_via_proto() {
            let config = test_load_config(FILENAME, "{}", |path| {
                let mut proto = ToolsConfig::default();
                proto.tools.insert(
                    Id::raw("deno"),
                    UnresolvedVersionSpec::parse("1.30.0").unwrap(),
                );

                ToolchainConfig::load_from(path, &proto)
            });

            assert!(config.deno.is_some());
            // assert_eq!(config.deno.unwrap().version.unwrap(), "1.30.0");
        }

        #[test]
        fn inherits_plugin_locator() {
            let config = test_load_config(FILENAME, "deno: {}", |path| {
                let mut tools = ToolsConfig::default();
                tools.inherit_builtin_plugins();

                ToolchainConfig::load_from(path, &tools)
            });

            assert_eq!(
                config.deno.unwrap().plugin.unwrap(),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/deno-plugin/releases/latest/download/deno_plugin.wasm".into()
                }
            );
        }
    }

    mod node {
        use super::*;

        #[test]
        fn uses_defaults() {
            let config = test_load_config(FILENAME, "node: {}", |path| {
                ToolchainConfig::load_from(path, &ToolsConfig::default())
            });

            let cfg = config.node.unwrap();

            assert!(cfg.dedupe_on_lockfile_change);
            assert!(!cfg.infer_tasks_from_scripts);
        }

        #[test]
        fn sets_values() {
            let config = test_load_config(
                FILENAME,
                r"
node:
  dedupeOnLockfileChange: false
  inferTasksFromScripts: true
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.node.unwrap();

            assert!(!cfg.dedupe_on_lockfile_change);
            assert!(cfg.infer_tasks_from_scripts);
        }

        #[test]
        fn enables_via_proto() {
            let config = test_load_config(FILENAME, "{}", |path| {
                let mut proto = ToolsConfig::default();
                proto.tools.insert(
                    Id::raw("node"),
                    UnresolvedVersionSpec::parse("18.0.0").unwrap(),
                );

                ToolchainConfig::load_from(path, &proto)
            });

            assert!(config.node.is_some());
            assert_eq!(config.node.unwrap().version.unwrap(), "18.0.0");
        }

        #[test]
        fn inherits_plugin_locator() {
            let config = test_load_config(FILENAME, "node: {}", |path| {
                let mut tools = ToolsConfig::default();
                tools.inherit_builtin_plugins();

                ToolchainConfig::load_from(path, &tools)
            });

            assert_eq!(
                config.node.unwrap().plugin.unwrap(),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_plugin.wasm".into()
                }
            );
        }

        #[test]
        fn proto_version_doesnt_override() {
            let config = test_load_config(
                FILENAME,
                r"
node:
  version: 20.0.0
",
                |path| {
                    let mut proto = ToolsConfig::default();
                    proto.tools.insert(
                        Id::raw("node"),
                        UnresolvedVersionSpec::parse("18.0.0").unwrap(),
                    );

                    ToolchainConfig::load_from(path, &proto)
                },
            );

            assert!(config.node.is_some());
            assert_eq!(config.node.unwrap().version.unwrap(), "20.0.0");
        }

        #[test]
        #[should_panic(expected = "not a valid semantic version")]
        fn validates_version() {
            test_load_config(
                FILENAME,
                r"
node:
  version: '1'
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );
        }

        #[test]
        fn inherits_version_from_env_var() {
            env::set_var("MOON_NODE_VERSION", "19.0.0");

            let config = test_load_config(
                FILENAME,
                r"
node:
  version: 20.0.0
",
                |path| {
                    let mut proto = ToolsConfig::default();
                    proto.tools.insert(
                        Id::raw("node"),
                        UnresolvedVersionSpec::parse("18.0.0").unwrap(),
                    );

                    ToolchainConfig::load_from(path, &proto)
                },
            );

            env::remove_var("MOON_NODE_VERSION");

            assert_eq!(config.node.unwrap().version.unwrap(), "19.0.0");
        }

        mod npm {
            use super::*;

            #[test]
            fn proto_version_doesnt_override() {
                let config = test_load_config(
                    FILENAME,
                    r"
node:
  npm:
    version: 9.0.0
",
                    |path| {
                        let mut proto = ToolsConfig::default();
                        proto.tools.insert(
                            Id::raw("npm"),
                            UnresolvedVersionSpec::parse("8.0.0").unwrap(),
                        );

                        ToolchainConfig::load_from(path, &proto)
                    },
                );

                assert_eq!(config.node.unwrap().npm.version.unwrap(), "9.0.0");
            }

            #[test]
            fn inherits_plugin_locator() {
                let config = test_load_config(FILENAME, "node:\n  npm: {}", |path| {
                    let mut tools = ToolsConfig::default();
                    tools.inherit_builtin_plugins();

                    ToolchainConfig::load_from(path, &tools)
                });

                assert_eq!(
                    config.node.unwrap().npm.plugin.unwrap(),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_depman_plugin.wasm".into()
                    }
                );
            }

            #[test]
            fn inherits_version_from_env_var() {
                env::set_var("MOON_NPM_VERSION", "10.0.0");

                let config = test_load_config(
                    FILENAME,
                    r"
node:
  npm:
    version: 9.0.0
",
                    |path| {
                        let mut proto = ToolsConfig::default();
                        proto.tools.insert(
                            Id::raw("npm"),
                            UnresolvedVersionSpec::parse("8.0.0").unwrap(),
                        );

                        ToolchainConfig::load_from(path, &proto)
                    },
                );

                env::remove_var("MOON_NPM_VERSION");

                assert_eq!(config.node.unwrap().npm.version.unwrap(), "10.0.0");
            }
        }

        mod pnpm {
            use super::*;

            #[test]
            fn enables_when_defined() {
                let config = test_load_config(FILENAME, "node: {}", |path| {
                    ToolchainConfig::load_from(path, &ToolsConfig::default())
                });

                assert!(config.node.unwrap().pnpm.is_none());

                let config = test_load_config(FILENAME, "node:\n  pnpm: {}", |path| {
                    ToolchainConfig::load_from(path, &ToolsConfig::default())
                });

                assert!(config.node.unwrap().pnpm.is_some());
            }

            #[test]
            fn inherits_plugin_locator() {
                let config = test_load_config(
                    FILENAME,
                    r"
node:
  packageManager: pnpm
  pnpm: {}
",
                    |path| {
                        let mut tools = ToolsConfig::default();
                        tools.inherit_builtin_plugins();

                        ToolchainConfig::load_from(path, &tools)
                    },
                );

                assert_eq!(
                    config.node.unwrap().pnpm.unwrap().plugin.unwrap(),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_depman_plugin.wasm".into()
                    }
                );
            }

            #[test]
            fn inherits_plugin_locator_when_none() {
                let config = test_load_config(
                    FILENAME,
                    r"
node:
  packageManager: pnpm
",
                    |path| {
                        let mut tools = ToolsConfig::default();
                        tools.inherit_builtin_plugins();

                        ToolchainConfig::load_from(path, &tools)
                    },
                );

                assert_eq!(
                    config.node.unwrap().pnpm.unwrap().plugin.unwrap(),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_depman_plugin.wasm".into()
                    }
                );
            }

            #[test]
            fn proto_version_doesnt_override() {
                let config = test_load_config(
                    FILENAME,
                    r"
node:
  pnpm:
    version: 9.0.0
",
                    |path| {
                        let mut proto = ToolsConfig::default();
                        proto.tools.insert(
                            Id::raw("pnpm"),
                            UnresolvedVersionSpec::parse("8.0.0").unwrap(),
                        );

                        ToolchainConfig::load_from(path, &proto)
                    },
                );

                assert_eq!(config.node.unwrap().pnpm.unwrap().version.unwrap(), "9.0.0");
            }

            #[test]
            fn inherits_version_from_env_var() {
                env::set_var("MOON_PNPM_VERSION", "10.0.0");

                let config = test_load_config(
                    FILENAME,
                    r"
node:
  pnpm:
    version: 9.0.0
",
                    |path| {
                        let mut proto = ToolsConfig::default();
                        proto.tools.insert(
                            Id::raw("pnpm"),
                            UnresolvedVersionSpec::parse("8.0.0").unwrap(),
                        );

                        ToolchainConfig::load_from(path, &proto)
                    },
                );

                env::remove_var("MOON_PNPM_VERSION");

                assert_eq!(
                    config.node.unwrap().pnpm.unwrap().version.unwrap(),
                    "10.0.0"
                );
            }
        }

        mod yarn {
            use super::*;

            #[test]
            fn enables_when_defined() {
                let config = test_load_config(FILENAME, "node: {}", |path| {
                    ToolchainConfig::load_from(path, &ToolsConfig::default())
                });

                assert!(config.node.unwrap().yarn.is_none());

                let config = test_load_config(FILENAME, "node:\n  yarn: {}", |path| {
                    ToolchainConfig::load_from(path, &ToolsConfig::default())
                });

                assert!(config.node.unwrap().yarn.is_some());
            }

            #[test]
            fn inherits_plugin_locator() {
                let config = test_load_config(
                    FILENAME,
                    r"
node:
  packageManager: yarn
  yarn: {}
",
                    |path| {
                        let mut tools = ToolsConfig::default();
                        tools.inherit_builtin_plugins();

                        ToolchainConfig::load_from(path, &tools)
                    },
                );

                assert_eq!(
                    config.node.unwrap().yarn.unwrap().plugin.unwrap(),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_depman_plugin.wasm".into()
                    }
                );
            }

            #[test]
            fn inherits_plugin_locator_when_none() {
                let config = test_load_config(
                    FILENAME,
                    r"
node:
  packageManager: yarn
",
                    |path| {
                        let mut tools = ToolsConfig::default();
                        tools.inherit_builtin_plugins();

                        ToolchainConfig::load_from(path, &tools)
                    },
                );

                assert_eq!(
                    config.node.unwrap().yarn.unwrap().plugin.unwrap(),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_depman_plugin.wasm".into()
                    }
                );
            }

            #[test]
            fn proto_version_doesnt_override() {
                let config = test_load_config(
                    FILENAME,
                    r"
node:
  yarn:
    version: 9.0.0
",
                    |path| {
                        let mut proto = ToolsConfig::default();
                        proto.tools.insert(
                            Id::raw("yarn"),
                            UnresolvedVersionSpec::parse("8.0.0").unwrap(),
                        );

                        ToolchainConfig::load_from(path, &proto)
                    },
                );

                assert_eq!(config.node.unwrap().yarn.unwrap().version.unwrap(), "9.0.0");
            }

            #[test]
            fn inherits_version_from_env_var() {
                env::set_var("MOON_YARN_VERSION", "10.0.0");

                let config = test_load_config(
                    FILENAME,
                    r"
node:
  yarn:
    version: 9.0.0
",
                    |path| {
                        let mut proto = ToolsConfig::default();
                        proto.tools.insert(
                            Id::raw("yarn"),
                            UnresolvedVersionSpec::parse("8.0.0").unwrap(),
                        );

                        ToolchainConfig::load_from(path, &proto)
                    },
                );

                env::remove_var("MOON_YARN_VERSION");

                assert_eq!(
                    config.node.unwrap().yarn.unwrap().version.unwrap(),
                    "10.0.0"
                );
            }
        }
    }

    mod rust {
        use super::*;

        #[test]
        fn uses_defaults() {
            let config = test_load_config(FILENAME, "rust: {}", |path| {
                ToolchainConfig::load_from(path, &ToolsConfig::default())
            });

            let cfg = config.rust.unwrap();

            assert!(cfg.bins.is_empty());
            assert!(!cfg.sync_toolchain_config);
        }

        #[test]
        fn sets_values() {
            let config = test_load_config(
                FILENAME,
                r"
rust:
  bins: [cargo-make]
  syncToolchainConfig: true
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.rust.unwrap();

            assert_eq!(cfg.bins, vec![BinEntry::Name("cargo-make".into())]);
            assert!(cfg.sync_toolchain_config);
        }

        #[test]
        fn can_set_bin_objects() {
            let config = test_load_config(
                FILENAME,
                r"
rust:
  bins:
    - cargo-make
    - bin: cargo-nextest
      name: 'next'
    - bin: cargo-insta
      local: true
  syncToolchainConfig: true
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.rust.unwrap();

            assert_eq!(
                cfg.bins,
                vec![
                    BinEntry::Name("cargo-make".into()),
                    BinEntry::Config(BinConfig {
                        bin: "cargo-nextest".into(),
                        name: Some("next".into()),
                        ..BinConfig::default()
                    }),
                    BinEntry::Config(BinConfig {
                        bin: "cargo-insta".into(),
                        local: true,
                        ..BinConfig::default()
                    }),
                ]
            );
            assert!(cfg.sync_toolchain_config);
        }

        #[test]
        fn enables_via_proto() {
            let config = test_load_config(FILENAME, "{}", |path| {
                let mut proto = ToolsConfig::default();
                proto.tools.insert(
                    Id::raw("rust"),
                    UnresolvedVersionSpec::parse("1.69.0").unwrap(),
                );

                ToolchainConfig::load_from(path, &proto)
            });

            assert!(config.rust.is_some());
            assert_eq!(config.rust.unwrap().version.unwrap(), "1.69.0");
        }

        #[test]
        fn inherits_plugin_locator() {
            let config = test_load_config(FILENAME, "rust: {}", |path| {
                let mut tools = ToolsConfig::default();
                tools.inherit_builtin_plugins();

                ToolchainConfig::load_from(path, &tools)
            });

            assert_eq!(
                config.rust.unwrap().plugin.unwrap(),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/rust-plugin/releases/latest/download/rust_plugin.wasm".into()
                }
            );
        }

        #[test]
        fn proto_version_doesnt_override() {
            let config = test_load_config(
                FILENAME,
                r"
rust:
  version: 1.60.0
",
                |path| {
                    let mut proto = ToolsConfig::default();
                    proto.tools.insert(
                        Id::raw("rust"),
                        UnresolvedVersionSpec::parse("1.69.0").unwrap(),
                    );

                    ToolchainConfig::load_from(path, &proto)
                },
            );

            assert!(config.rust.is_some());
            assert_eq!(config.rust.unwrap().version.unwrap(), "1.60.0");
        }

        #[test]
        #[should_panic(expected = "not a valid semantic version")]
        fn validates_version() {
            test_load_config(
                FILENAME,
                r"
rust:
  version: '1'
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );
        }

        #[test]
        fn inherits_version_from_env_var() {
            env::set_var("MOON_RUST_VERSION", "1.70.0");

            let config = test_load_config(
                FILENAME,
                r"
        rust:
          version: 1.60.0
        ",
                |path| {
                    let mut proto = ToolsConfig::default();
                    proto.tools.insert(
                        Id::raw("rust"),
                        UnresolvedVersionSpec::parse("1.65.0").unwrap(),
                    );

                    ToolchainConfig::load_from(path, &proto)
                },
            );

            env::remove_var("MOON_RUST_VERSION");

            assert_eq!(config.rust.unwrap().version.unwrap(), "1.70.0");
        }
    }

    mod typescript {
        use super::*;

        #[test]
        fn uses_defaults() {
            let config = test_load_config(FILENAME, "typescript: {}", |path| {
                ToolchainConfig::load_from(path, &ToolsConfig::default())
            });

            let cfg = config.typescript.unwrap();

            assert_eq!(cfg.project_config_file_name, "tsconfig.json".to_owned());
            assert!(cfg.sync_project_references);
        }

        #[test]
        fn sets_values() {
            let config = test_load_config(
                FILENAME,
                r"
typescript:
  projectConfigFileName: tsconf.json
  syncProjectReferences: false
",
                |path| ToolchainConfig::load_from(path, &ToolsConfig::default()),
            );

            let cfg = config.typescript.unwrap();

            assert_eq!(cfg.project_config_file_name, "tsconf.json".to_owned());
            assert!(!cfg.sync_project_references);
        }

        #[test]
        fn enables_via_proto() {
            let config = test_load_config(FILENAME, "{}", |path| {
                let mut proto = ToolsConfig::default();
                proto.tools.insert(
                    Id::raw("typescript"),
                    UnresolvedVersionSpec::parse("5.0.0").unwrap(),
                );

                ToolchainConfig::load_from(path, &proto)
            });

            assert!(config.typescript.is_some());
            // assert_eq!(config.typescript.unwrap().version.unwrap(), "1.30.0");
        }
    }
}
