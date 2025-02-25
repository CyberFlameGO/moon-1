use clap::Args;
use console::Term;
use miette::{miette, IntoDiagnostic};
use moon::build_project_graph;
use moon_target::Target;
use moon_terminal::{ExtendedTerm, Label};
use moon_workspace::Workspace;
use starbase::system;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct TaskArgs {
    #[arg(help = "Target of task to display")]
    target: Target,

    #[arg(long, help = "Print in JSON format")]
    json: bool,
}

#[system]
pub async fn task(args: ArgsRef<TaskArgs>, workspace: ResourceMut<Workspace>) {
    let Some(project_id) = args.target.scope_id.clone() else {
        return Err(miette!("A project ID is required."));
    };

    let mut project_graph_builder = build_project_graph(workspace).await?;
    project_graph_builder.load(&project_id).await?;

    let project_graph = project_graph_builder.build().await?;
    let project = project_graph.get(&project_id)?;
    let task = project.get_task(&args.target.task_id)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&task).into_diagnostic()?);

        return Ok(());
    }

    let term = Term::buffered_stdout();

    term.line("")?;
    term.render_label(Label::Brand, &args.target.id)?;
    term.render_entry("Task", color::id(&args.target.task_id))?;
    term.render_entry("Project", color::id(&project_id))?;
    term.render_entry("Platform", term.format(&task.platform))?;
    term.render_entry("Type", term.format(&task.type_of))?;

    term.line("")?;
    term.render_label(Label::Default, "Process")?;
    term.render_entry(
        "Command",
        color::shell(format!("{} {}", task.command, task.args.join(" "))),
    )?;

    if !task.env.is_empty() {
        term.render_entry_list(
            "Environment variables",
            task.env
                .iter()
                .map(|(k, v)| format!("{} {} {}", k, color::muted_light("="), v))
                .collect::<Vec<_>>(),
        )?;
    }

    term.render_entry(
        "Working directory",
        color::path(if task.options.run_from_workspace_root {
            &workspace.root
        } else {
            &project.root
        }),
    )?;
    term.render_entry(
        "Runs dependencies",
        if task.options.run_deps_in_parallel {
            "Concurrently"
        } else {
            "Serially"
        },
    )?;
    term.render_entry_bool("Runs in CI", task.should_run_in_ci())?;

    if !task.deps.is_empty() {
        term.line("")?;
        term.render_label(Label::Default, "Depends on")?;
        term.render_list(task.deps.iter().map(color::label).collect::<Vec<_>>())?;
    }

    if !task.input_files.is_empty() || !task.input_globs.is_empty() {
        term.line("")?;
        term.render_label(Label::Default, "Inputs")?;
        term.render_list(
            task.input_globs
                .iter()
                .map(color::rel_path)
                .collect::<Vec<_>>(),
        )?;
        term.render_list(
            task.input_files
                .iter()
                .map(color::rel_path)
                .collect::<Vec<_>>(),
        )?;
    }

    if !task.output_files.is_empty() || !task.output_globs.is_empty() {
        term.line("")?;
        term.render_label(Label::Default, "Outputs")?;
        term.render_list(
            task.output_globs
                .iter()
                .map(color::rel_path)
                .collect::<Vec<_>>(),
        )?;
        term.render_list(
            task.output_files
                .iter()
                .map(color::rel_path)
                .collect::<Vec<_>>(),
        )?;
    }

    term.line("")?;
    term.flush_lines()?;
}
