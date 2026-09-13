use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use miette::{IntoDiagnostic, WrapErr};

#[derive(Parser)]
struct Cli {
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    #[arg(short, long, conflicts_with = "parse")]
    lex: bool,

    #[arg(short, long)]
    parse: bool,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let source = read_source(cli.file.as_deref())?;

    if cli.lex {
        return run_lex(&source);
    }
    if cli.parse {
        return run_parse(&source);
    }

    let tokens = monocle::lex(&source)?;
    let expressions = monocle::parse(tokens, &source)?;
    print!("{}", monocle::codegen(expressions));

    Ok(())
}

fn read_source(path: Option<&Path>) -> miette::Result<String> {
    if let Some(path) = path {
        return fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to read {}", path.display()));
    }

    Ok("(1 + (2 + 3))".to_string())
}

fn run_lex(source: &str) -> miette::Result<()> {
    let tokens = monocle::lex(source)?;
    let kinds: Vec<_> = tokens.iter().map(|token| &token.kind).collect();
    println!("{kinds:#?}");
    Ok(())
}

fn run_parse(source: &str) -> miette::Result<()> {
    let tokens = monocle::lex(source)?;
    let expressions = monocle::parse(tokens, source)?;
    println!("{expressions:#?}");
    Ok(())
}
