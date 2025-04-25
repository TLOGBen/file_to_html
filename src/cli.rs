use std::io;
use clap::Parser;
use crate::config::{Cli, Mode, PasswordMode, validate_cli_args};
use crate::interactive::{process_interactive_mode, prompt_manual_password};
use crate::utils::{setup_logging, create_regex_sets};
use crate::convert::{process_individual, process_compressed};

pub fn process_args(args: Vec<String>) -> io::Result<String> {
    if args.len() == 1 {
        process_interactive_mode()
    } else {
        process_cli_mode()
    }
}

pub fn process_cli_mode() -> io::Result<String> {
    let cli = Cli::parse();
    validate_cli_args(&cli)?;
    setup_logging(&cli.log_level)?;

    let (include_set, exclude_set) = create_regex_sets(&cli.include, &cli.exclude.as_deref().unwrap_or(&[]).to_vec());
    let display_password = cli.display_password.unwrap_or_else(|| cli.password_mode == "random");
    let password_mode = match cli.password_mode.as_str() {
        "random" => PasswordMode::Random,
        "manual" => PasswordMode::Manual,
        "timestamp" => PasswordMode::Timestamp,
        "none" => PasswordMode::None,
        _ => PasswordMode::Random,
    };

    let preset_password = if cli.password_mode == "manual" {
        Some(prompt_manual_password()?)
    } else {
        None
    };

    match cli.mode {
        Mode::Individual => {
            log::info!("開始個別轉換，輸入路徑：{}，輸出目錄：{}，包含模式：{:?}",
                  cli.input, cli.output, cli.include);
            process_individual(
                std::path::Path::new(&cli.input),
                &cli.output,
                &include_set,
                &exclude_set,
                cli.compress,
                &cli.compression_level,
                password_mode,
                display_password,
                &cli.layer,
                &cli.encryption_method,
                cli.no_progress,
                cli.max_size,
                preset_password,
            )?;
        }
        Mode::Compressed => {
            log::info!("開始壓縮轉換，輸入路徑：{}，輸出目錄：{}，包含模式：{:?}",
                  cli.input, cli.output, cli.include);
            process_compressed(
                std::path::Path::new(&cli.input),
                &cli.output,
                &include_set,
                &exclude_set,
                password_mode,
                display_password,
                &cli.compression_level,
                &cli.layer,
                &cli.encryption_method,
                cli.no_progress,
                cli.max_size,
                preset_password,
            )?;
        }
    }

    Ok(cli.output)
}