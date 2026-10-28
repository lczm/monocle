use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
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

    #[arg(short, long, conflicts_with_all = ["lex", "parse"], requires = "file")]
    run: bool,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    if cli.run {
        return run_program(cli.file.as_deref().expect("--run requires a file"));
    }

    let source = read_source(cli.file.as_deref())?;

    if cli.lex {
        return run_lex(&source);
    }
    if cli.parse {
        return run_parse(&source);
    }

    let tokens = monocle::lex(&source)?;
    let statements = monocle::parse(tokens, &source)?;

    // analysis step
    monocle::analyze(&statements, &source)?;

    print!("{}", monocle::codegen(statements)?);

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

const OUT_ASM: &str = "out.s";
const OUT_BIN: &str = "out";
const CC: &str = "riscv64-linux-gnu-gcc";
const QEMU: &str = "qemu-riscv64";

fn run_program(file: &Path) -> miette::Result<()> {
    // monocle will compile into assembly
    // use CC to link from assembly into a static binary
    tool_exists(CC)?;
    // qemu will run the static binary
    tool_exists(QEMU)?;

    let source = read_source(Some(file))?;
    let tokens = monocle::lex(&source)?;
    let statements = monocle::parse(tokens, &source)?;
    monocle::analyze(&statements, &source)?;
    let assembly = monocle::codegen(statements)?;

    // write it out to a file and link it separately
    fs::write(OUT_ASM, &assembly)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write {OUT_ASM}"))?;

    let link = Command::new(CC)
        .args(["-static", OUT_ASM, "-o", OUT_BIN])
        .status()
        .into_diagnostic()?;
    if !link.success() {
        // gcc already printed its diagnostics; point at that output.
        return Err(miette::miette!("linking failed (see {CC} output above)"));
    }

    let exit = Command::new(QEMU).arg(OUT_BIN).status().into_diagnostic()?;
    let code = exit.code().unwrap_or(-1);
    println!("exit code: {code}");
    Ok(())
}

fn tool_exists(tool: &str) -> miette::Result<()> {
    let found = Command::new(tool)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if found {
        Ok(())
    } else {
        Err(miette::miette!(
            "`{tool}` not found on PATH; --run needs the RISC-V cross toolchain. \
             Install e.g. `gcc-riscv64-linux-gnu` and `qemu-user`."
        ))
    }
}
