use crate::validate::validate_semver;
use crate::{inherit_tool, inherit_tool_required};
use proto_core::{PluginLocator, ToolsConfig};
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum NodeProjectAliasFormat {
        #[default]
        NameAndScope, // @scope/name
        NameOnly, // name
    }
);

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum NodeVersionFormat {
        File,         // file:..
        Link,         // link:..
        Star,         // *
        Version,      // 0.0.0
        VersionCaret, // ^0.0.0
        VersionTilde, // ~0.0.0
        #[default]
        Workspace, // workspace:*
        WorkspaceCaret, // workspace:^
        WorkspaceTilde, // workspace:~
    }
);

impl NodeVersionFormat {
    pub fn get_prefix(&self) -> String {
        match self {
            NodeVersionFormat::File => "file:".into(),
            NodeVersionFormat::Link => "link:".into(),
            NodeVersionFormat::Star => "*".into(),
            NodeVersionFormat::Version => "".into(),
            NodeVersionFormat::VersionCaret => "^".into(),
            NodeVersionFormat::VersionTilde => "~".into(),
            NodeVersionFormat::Workspace => "workspace:*".into(),
            NodeVersionFormat::WorkspaceCaret => "workspace:^".into(),
            NodeVersionFormat::WorkspaceTilde => "workspace:~".into(),
        }
    }
}

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum NodePackageManager {
        #[default]
        Npm,
        Pnpm,
        Yarn,
    }
);

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum NodeVersionManager {
        Nodenv,
        #[default]
        Nvm,
    }
);

#[derive(Clone, Config, Debug)]
pub struct NpmConfig {
    pub plugin: Option<PluginLocator>,

    #[setting(env = "MOON_NPM_VERSION", validate = validate_semver)]
    pub version: Option<String>,
}

#[derive(Clone, Config, Debug)]
pub struct PnpmConfig {
    pub plugin: Option<PluginLocator>,

    #[setting(env = "MOON_PNPM_VERSION", validate = validate_semver)]
    pub version: Option<String>,
}

#[derive(Clone, Config, Debug)]
pub struct YarnConfig {
    pub plugin: Option<PluginLocator>,

    pub plugins: Vec<String>,

    #[setting(env = "MOON_YARN_VERSION", validate = validate_semver)]
    pub version: Option<String>,
}

/// Docs: https://moonrepo.dev/docs/config/toolchain#node
#[derive(Clone, Config, Debug)]
pub struct NodeConfig {
    #[setting(default = true)]
    pub add_engines_constraint: bool,

    #[deprecated]
    pub alias_package_names: NodeProjectAliasFormat,

    pub bin_exec_args: Vec<String>,

    #[setting(default = true)]
    pub dedupe_on_lockfile_change: bool,

    pub dependency_version_format: NodeVersionFormat,

    pub infer_tasks_from_scripts: bool,

    #[setting(nested)]
    pub npm: NpmConfig,

    pub package_manager: NodePackageManager,

    pub plugin: Option<PluginLocator>,

    #[setting(nested)]
    pub pnpm: Option<PnpmConfig>,

    #[setting(default = true)]
    pub sync_project_workspace_dependencies: bool,

    pub sync_version_manager_config: Option<NodeVersionManager>,

    #[setting(env = "MOON_NODE_VERSION", validate = validate_semver)]
    pub version: Option<String>,

    #[setting(nested)]
    pub yarn: Option<YarnConfig>,
}

impl NodeConfig {
    inherit_tool_required!(NpmConfig, npm, "npm", inherit_proto_npm);
    inherit_tool!(PnpmConfig, pnpm, "pnpm", inherit_proto_pnpm);
    inherit_tool!(YarnConfig, yarn, "yarn", inherit_proto_yarn);

    pub fn inherit_proto(&mut self, proto_tools: &ToolsConfig) -> miette::Result<()> {
        match &self.package_manager {
            NodePackageManager::Npm => {
                self.inherit_proto_npm(proto_tools)?;
            }
            NodePackageManager::Pnpm => {
                if self.pnpm.is_none() {
                    self.pnpm = Some(PnpmConfig::default());
                }

                self.inherit_proto_pnpm(proto_tools)?;
            }
            NodePackageManager::Yarn => {
                if self.yarn.is_none() {
                    self.yarn = Some(YarnConfig::default());
                }

                self.inherit_proto_yarn(proto_tools)?;
            }
        };

        if self.plugin.is_none() {
            self.plugin = proto_tools.plugins.get("node").cloned();
        }

        Ok(())
    }
}
