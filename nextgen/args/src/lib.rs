use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[diagnostic(code(args::split))]
#[error("Failed to split arguments \"{args}\".")]
pub struct ArgsSplitError {
    args: String,
    #[source]
    error: shell_words::ParseError,
}

// When parsing a command line with multiple commands separated by a semicolon,
// like "mkdir foo; cd foo", the semicolon is considered part of the leading argument
// if there's no space between them. This attempts to pad the space.
fn pad_semicolon(line: &str) -> String {
    line.replace("; ", " ; ")
}

pub fn split_args<T: AsRef<str>>(line: T) -> miette::Result<Vec<String>> {
    let line = pad_semicolon(line.as_ref());

    Ok(shell_words::split(&line).map_err(|error| ArgsSplitError {
        args: line.to_owned(),
        error,
    })?)
}

// #[cfg(windows)]
// pub fn split_args<T: AsRef<str>>(line: T) -> miette::Result<Vec<String>> {
//     let line = pad_semicolon(line.as_ref());

//     Ok(winsplit::split(&line))
// }

// Using `shell_words::join` here incorrectly quotes ";" and other
// characters, breaking multi-commands.
pub fn join_args<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut line = args.into_iter().fold(String::new(), |mut line, arg| {
        let arg = arg.as_ref();

        match arg {
            "&" | "&&" | "|&" | "|" | "||" | ";" | "!" | ">" | ">>" | "<" | "-" | "--" => {
                line.push_str(arg);
                line.push(' ');
            }
            _ => {
                if arg.starts_with('$') || arg.starts_with('\'') || arg.starts_with('"') {
                    line.push_str(arg);
                } else {
                    let quoted = shell_words::quote(arg);
                    line.push_str(quoted.as_ref());
                }

                line.push(' ');
            }
        };

        line
    });

    line.pop();
    line
}
