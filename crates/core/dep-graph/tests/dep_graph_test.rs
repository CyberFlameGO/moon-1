use moon::{build_dep_graph, generate_project_graph, load_workspace_from};
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{
    PartialInheritedTasksConfig, PartialNodeConfig, PartialToolchainConfig, PartialWorkspaceConfig,
    PartialWorkspaceProjects,
};
use moon_dep_graph::BatchedTopoSort;
use moon_project_graph::ProjectGraph;
use moon_target::Target;
use moon_test_utils::{assert_snapshot, create_input_paths, create_sandbox_with_config, Sandbox};
use moon_workspace::Workspace;
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};

async fn create_project_graph() -> (Workspace, ProjectGraph, Sandbox) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("advanced".into(), "advanced".to_owned()),
            ("basic".into(), "basic".to_owned()),
            ("emptyConfig".into(), "empty-config".to_owned()),
            ("noConfig".into(), "no-config".to_owned()),
            // Deps
            ("foo".into(), "deps/foo".to_owned()),
            ("bar".into(), "deps/bar".to_owned()),
            ("baz".into(), "deps/baz".to_owned()),
            // Tasks
            ("tasks".into(), "tasks".to_owned()),
            ("platforms".into(), "platforms".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };
    let toolchain_config = PartialToolchainConfig {
        node: Some(PartialNodeConfig {
            version: Some("16.0.0".into()),
            dedupe_on_lockfile_change: Some(false),
            ..PartialNodeConfig::default()
        }),
        ..PartialToolchainConfig::default()
    };
    let tasks_config = PartialInheritedTasksConfig {
        file_groups: Some(FxHashMap::from_iter([
            (
                "sources".into(),
                create_input_paths(["src/**/*", "types/**/*"]),
            ),
            ("tests".into(), create_input_paths(["tests/**/*"])),
        ])),
        ..PartialInheritedTasksConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "projects",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let project_graph = generate_project_graph(&mut workspace).await.unwrap();

    (workspace, project_graph, sandbox)
}

async fn create_tasks_project_graph() -> (Workspace, ProjectGraph, Sandbox) {
    let workspace_config = PartialWorkspaceConfig {
        projects: Some(PartialWorkspaceProjects::Sources(FxHashMap::from_iter([
            ("basic".into(), "basic".to_owned()),
            ("buildA".into(), "build-a".to_owned()),
            ("buildB".into(), "build-b".to_owned()),
            ("buildC".into(), "build-c".to_owned()),
            ("chain".into(), "chain".to_owned()),
            ("cycle".into(), "cycle".to_owned()),
            ("inputA".into(), "input-a".to_owned()),
            ("inputB".into(), "input-b".to_owned()),
            ("inputC".into(), "input-c".to_owned()),
            (
                "mergeAllStrategies".into(),
                "merge-all-strategies".to_owned(),
            ),
            ("mergeAppend".into(), "merge-append".to_owned()),
            ("mergePrepend".into(), "merge-prepend".to_owned()),
            ("mergeReplace".into(), "merge-replace".to_owned()),
            ("noTasks".into(), "no-tasks".to_owned()),
            ("persistent".into(), "persistent".to_owned()),
            ("interactive".into(), "interactive".to_owned()),
        ]))),
        ..PartialWorkspaceConfig::default()
    };
    let toolchain_config = PartialToolchainConfig {
        node: Some(PartialNodeConfig {
            version: Some("16.0.0".into()),
            ..PartialNodeConfig::default()
        }),
        ..PartialToolchainConfig::default()
    };
    let tasks_config = PartialInheritedTasksConfig {
        file_groups: Some(FxHashMap::from_iter([(
            "sources".into(),
            create_input_paths(["src/**/*"]),
        )])),
        ..PartialInheritedTasksConfig::default()
    };

    let sandbox = create_sandbox_with_config(
        "tasks",
        Some(workspace_config),
        Some(toolchain_config),
        Some(tasks_config),
    );

    let mut workspace = load_workspace_from(sandbox.path()).await.unwrap();
    let project_graph = generate_project_graph(&mut workspace).await.unwrap();

    (workspace, project_graph, sandbox)
}

fn sort_batches(batches: BatchedTopoSort) -> BatchedTopoSort {
    let mut list: BatchedTopoSort = vec![];

    for batch in batches {
        let mut new_batch = batch.clone();
        new_batch.sort();
        list.push(new_batch);
    }

    list
}

#[tokio::test]
#[should_panic(expected = "A dependency cycle has been detected for RunTarget(cycle:a)")]
async fn detects_cycles() {
    let (_workspace, projects, _sandbox) = create_tasks_project_graph().await;

    let mut graph = build_dep_graph(&projects);
    graph
        .run_target(&Target::new("cycle", "a").unwrap(), None)
        .unwrap();
    graph
        .run_target(&Target::new("cycle", "b").unwrap(), None)
        .unwrap();
    graph
        .run_target(&Target::new("cycle", "c").unwrap(), None)
        .unwrap();
    let graph = graph.build();

    assert_eq!(
        sort_batches(graph.sort_batched_topological().unwrap()),
        vec![vec![NodeIndex::new(0)], vec![NodeIndex::new(1)]]
    );
}

mod run_target {
    use super::*;

    #[tokio::test]
    async fn single_targets() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("tasks", "test").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), None)
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2), // sync project
                NodeIndex::new(3), // test
                NodeIndex::new(4), // lint
                NodeIndex::new(5),
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(1)],
                vec![NodeIndex::new(2), NodeIndex::new(3)],
                vec![NodeIndex::new(0), NodeIndex::new(4), NodeIndex::new(5)]
            ]
        );
    }

    #[tokio::test]
    async fn deps_chain_target() {
        let (_workspace, projects, _sandbox) = create_tasks_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("basic", "test").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("basic", "lint").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("chain", "a").unwrap(), None)
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2), // sync project
                NodeIndex::new(3), // test
                NodeIndex::new(4), // lint
                NodeIndex::new(5), // sync project
                NodeIndex::new(6),
                NodeIndex::new(12), // f
                NodeIndex::new(11), // e
                NodeIndex::new(10), // d
                NodeIndex::new(9),  // c
                NodeIndex::new(8),  // b
                NodeIndex::new(7),  // a
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(1)],
                vec![NodeIndex::new(2), NodeIndex::new(6)],
                vec![NodeIndex::new(12)],
                vec![NodeIndex::new(11)],
                vec![NodeIndex::new(10)],
                vec![NodeIndex::new(9)],
                vec![NodeIndex::new(3), NodeIndex::new(8)],
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(4),
                    NodeIndex::new(5),
                    NodeIndex::new(7)
                ]
            ]
        );
    }

    #[tokio::test]
    async fn moves_persistent_tasks_last() {
        let (_workspace, projects, _sandbox) = create_tasks_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("persistent", "dev").unwrap(), None)
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(1)],
                vec![
                    NodeIndex::new(2),
                    NodeIndex::new(5),
                    NodeIndex::new(6),
                    NodeIndex::new(7)
                ],
                vec![NodeIndex::new(4), NodeIndex::new(12), NodeIndex::new(13)],
                vec![NodeIndex::new(3), NodeIndex::new(11)],
                vec![NodeIndex::new(0)],
                vec![
                    NodeIndex::new(8),
                    NodeIndex::new(9),
                    NodeIndex::new(10),
                    NodeIndex::new(14)
                ],
            ]
        );
    }

    #[tokio::test]
    async fn isolates_interactive_tasks() {
        let (_workspace, projects, _sandbox) = create_tasks_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("interactive", "one").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("chain", "c").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("interactive", "two").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("basic", "lint").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("interactive", "depOnOne").unwrap(), None)
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(1)],
                vec![NodeIndex::new(2), NodeIndex::new(5)],
                vec![NodeIndex::new(9)],
                vec![NodeIndex::new(3), NodeIndex::new(8)],
                vec![NodeIndex::new(4)], // interactive
                vec![NodeIndex::new(7), NodeIndex::new(11)],
                vec![NodeIndex::new(13)], // interactive
                vec![NodeIndex::new(10)], // interactive
                vec![NodeIndex::new(0), NodeIndex::new(6), NodeIndex::new(12)],
            ]
        );
    }

    #[tokio::test]
    async fn avoids_dupe_targets() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), None)
            .unwrap();
        graph
            .run_target(&Target::new("tasks", "lint").unwrap(), None)
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2), // sync project
                NodeIndex::new(3), // lint
                NodeIndex::new(4),
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(1)],
                vec![NodeIndex::new(2), NodeIndex::new(3)],
                vec![NodeIndex::new(0), NodeIndex::new(4)]
            ]
        );
    }

    #[tokio::test]
    async fn runs_all_projects_for_target_all_scope() {
        let (_workspace, projects, _sandbox) = create_tasks_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::parse(":build").unwrap(), None)
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1),
                NodeIndex::new(2),  // sync project: basic
                NodeIndex::new(4),  // basic:build
                NodeIndex::new(5),  // sync project: build-c
                NodeIndex::new(6),  // sync project: build-a
                NodeIndex::new(3),  // build-c:build
                NodeIndex::new(8),  // build-a:build
                NodeIndex::new(9),  // sync project: build-b
                NodeIndex::new(7),  // build-b:build
                NodeIndex::new(10), // notasks
                NodeIndex::new(11)
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(1)],
                vec![
                    NodeIndex::new(2),
                    NodeIndex::new(4),
                    NodeIndex::new(5),
                    NodeIndex::new(6)
                ],
                vec![
                    NodeIndex::new(3),
                    NodeIndex::new(8),
                    NodeIndex::new(9),
                    NodeIndex::new(10)
                ],
                vec![NodeIndex::new(0), NodeIndex::new(7), NodeIndex::new(11)],
            ]
        );
    }

    #[tokio::test]
    #[should_panic(expected = "Dependencies scope (^:) is not supported in run contexts.")]
    async fn errors_for_target_deps_scope() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::parse("^:lint").unwrap(), None)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "Self scope (~:) is not supported in run contexts.")]
    async fn errors_for_target_self_scope() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::parse("~:lint").unwrap(), None)
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "No project has been configured with the name or alias unknown")]
    async fn errors_for_unknown_project() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("unknown", "test").unwrap(), None)
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    #[should_panic(expected = "Unknown task build for project tasks.")]
    async fn errors_for_unknown_task() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("tasks", "build").unwrap(), None)
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
    }
}

mod run_target_if_touched {
    use super::*;

    #[tokio::test]
    async fn skips_if_untouched_project() {
        let (_workspace, projects, _sandbox) = create_tasks_project_graph().await;

        let mut touched_files = FxHashSet::default();
        touched_files.insert(WorkspaceRelativePathBuf::from("input-a/a.ts"));
        touched_files.insert(WorkspaceRelativePathBuf::from("input-c/c.ts"));

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("inputA", "a").unwrap(), Some(&touched_files))
            .unwrap();
        graph
            .run_target(&Target::new("inputB", "b").unwrap(), Some(&touched_files))
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    async fn skips_if_untouched_task() {
        let (_workspace, projects, _sandbox) = create_tasks_project_graph().await;

        let mut touched_files = FxHashSet::default();
        touched_files.insert(WorkspaceRelativePathBuf::from("input-a/a2.ts"));
        touched_files.insert(WorkspaceRelativePathBuf::from("input-b/b2.ts"));
        touched_files.insert(WorkspaceRelativePathBuf::from("input-c/any.ts"));

        let mut graph = build_dep_graph(&projects);
        graph
            .run_target(&Target::new("inputA", "a").unwrap(), Some(&touched_files))
            .unwrap();
        graph
            .run_target(&Target::new("inputB", "b2").unwrap(), Some(&touched_files))
            .unwrap();
        graph
            .run_target(&Target::new("inputC", "c").unwrap(), Some(&touched_files))
            .unwrap();
        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
    }
}

mod sync_project {
    use super::*;
    use moon_dep_graph::DepGraphBuilder;

    fn sync_projects(graph: &mut DepGraphBuilder, projects: &ProjectGraph, ids: &[&str]) {
        for id in ids {
            let project = projects.get(id).unwrap();

            graph.sync_project(&project).unwrap();
        }
    }

    #[tokio::test]
    async fn isolated_projects() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;
        let mut graph = build_dep_graph(&projects);

        sync_projects(
            &mut graph,
            &projects,
            &["advanced", "basic", "emptyConfig", "noConfig"],
        );

        let graph = graph.build();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1), // advanced
                NodeIndex::new(2), // noConfig
                NodeIndex::new(4),
                NodeIndex::new(5), // basic
                NodeIndex::new(3), // emptyConfig
                NodeIndex::new(6),
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(4)],
                vec![NodeIndex::new(1), NodeIndex::new(5)],
                vec![
                    NodeIndex::new(0),
                    NodeIndex::new(2),
                    NodeIndex::new(3),
                    NodeIndex::new(6)
                ]
            ]
        );
    }

    #[tokio::test]
    async fn projects_with_deps() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;
        let mut graph = build_dep_graph(&projects);

        sync_projects(&mut graph, &projects, &["foo", "bar", "baz", "basic"]);

        let graph = graph.build();

        // Not deterministic!
        // assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1), // baz
                NodeIndex::new(3), // bar
                NodeIndex::new(4),
                NodeIndex::new(5), // foo
                NodeIndex::new(2), // noConfig
                NodeIndex::new(7), // basic
                NodeIndex::new(6),
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(3)],
                vec![
                    NodeIndex::new(1),
                    NodeIndex::new(4),
                    NodeIndex::new(5),
                    NodeIndex::new(7)
                ],
                vec![NodeIndex::new(0), NodeIndex::new(2), NodeIndex::new(6)]
            ]
        );
    }

    #[tokio::test]
    async fn projects_with_tasks() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;
        let mut graph = build_dep_graph(&projects);

        sync_projects(&mut graph, &projects, &["noConfig", "tasks"]);

        let graph = graph.build();

        assert_snapshot!(graph.to_dot());

        assert_eq!(
            graph.sort_topological().unwrap(),
            vec![
                NodeIndex::new(0),
                NodeIndex::new(1), // noConfig
                NodeIndex::new(2),
                NodeIndex::new(3), // tasks
                NodeIndex::new(4),
            ]
        );
        assert_eq!(
            sort_batches(graph.sort_batched_topological().unwrap()),
            vec![
                vec![NodeIndex::new(1), NodeIndex::new(3)],
                vec![NodeIndex::new(0), NodeIndex::new(2), NodeIndex::new(4)]
            ]
        );
    }

    #[tokio::test]
    async fn avoids_dupe_projects() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;
        let mut graph = build_dep_graph(&projects);

        sync_projects(&mut graph, &projects, &["advanced", "advanced", "advanced"]);

        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
    }

    #[tokio::test]
    #[should_panic(expected = "No project has been configured with the name or alias unknown")]
    async fn errors_for_unknown_project() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;
        let mut graph = build_dep_graph(&projects);

        sync_projects(&mut graph, &projects, &["unknown"]);

        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
    }
}

mod installs_deps {
    use super::*;

    #[tokio::test]
    async fn tool_is_based_on_task_platform() {
        let (_workspace, projects, _sandbox) = create_project_graph().await;
        let mut graph = build_dep_graph(&projects);

        graph
            .run_target(&Target::new("platforms", "system").unwrap(), None)
            .unwrap();

        graph
            .run_target(&Target::new("platforms", "node").unwrap(), None)
            .unwrap();

        let graph = graph.build();

        assert_snapshot!(graph.to_dot());
    }
}
