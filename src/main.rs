#![feature(let_chains)]

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::time::Instant;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use clap::{Parser, Subcommand, ValueHint};
use colored::Colorize;
use fancy_regex::Regex;
use glob::glob;
use path_absolutize::*;

use crate::config::Config;
use crate::expression::Expression;

mod expression;
mod config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Precompile Swift source files")]
    Precompile {
        /// Directory with Swift source files
        #[arg(default_value = "./", value_hint = ValueHint::DirPath)]
        directory: String,

        /// Output Swift file with precompiled code
        #[arg(short, long, default_value = "./Precompiled.swift", value_hint = ValueHint::FilePath)]
        out: String,

        /// Precompile without generating output file
        #[arg(long = "dry-run")]
        dry_run: bool,

        /// Clean output file before precompiling
        #[arg(long = "clean")]
        clean: bool,

        #[arg(long = "xcode-script-renderer")]
        xcode_script_renderer: bool,

        #[arg(long, default_value = "./swift-precompiled.toml", value_hint = ValueHint::FilePath)]
        config: String
    },
    #[command(about = "Clean precompiled file")]
    Clean {
        /// Output Swift file with precompiled code
        #[arg(short, long, default_value = "./Precompiled.swift", value_hint = ValueHint::FilePath)]
        out: String
    },
    Init {
        /// Output Swift file with precompiled code
        #[arg(long, default_value = "./swift-precompiled.toml", value_hint = ValueHint::FilePath)]
        config: String
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.commands {
        Commands::Precompile {
            mut directory,
            out,
            dry_run,
            clean,
            xcode_script_renderer,
            config
        } => {
            let precompile_time = Instant::now();

            let config_path = Path::new(&config);
            let mut precompiled_config = Config::default();
            if config_path.exists() {
                let mut config_file = File::open(&config_path).expect("Unable to open config file");
                let mut config_file_data = String::new();

                config_file.read_to_string(&mut config_file_data).expect("Unable to read config file");
                precompiled_config = toml::from_str(&config_file_data).expect("Unable to parse config file");

                if let Some(dirs) = precompiled_config.dirs {
                    directory = dirs.iter()
                        .map(|dir| dir.to_string())
                        // error[E0515]: cannot return value referencing temporary value &dir
                        .map(|dir| PathBuf::from(dir).absolutize().unwrap().to_path_buf())
                        .filter(|dir| dir.exists() && dir.is_dir())
                        .map(|dir| dir.display().to_string())
                        .collect::<Vec<String>>().join(":");
                }

                config_file_data.clear();
            }

            let out_path = Path::new(&out);
            let out_abs_path = out_path
                .absolutize()
                .unwrap()
                .display()
                .to_string()
                .to_owned();
            if out_path.is_dir() {
                println!("{} already exists as a directory", out_abs_path);
                return;
            }

            let mut precompile_file: Option<File> = None;
            let mut precompile_file_data = String::new();
            if !dry_run {
                precompile_file = Option::from(
                    OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .read(true)
                        .open(&out_path)
                        .expect(&*format!("Unable to create source file {}", out_abs_path)),
                ).map(|file| {
                    if clean {
                        file.set_len(0).expect("Unable to clean precompiled file");
                    }
                    file
                });

                if let Some(precompile_file) = precompile_file.as_mut() {
                    precompile_file.write_all(include_bytes!("../assets/PrecompiledTemplate.swift")).expect("Failed to write precompiled template");
                    precompile_file.seek(std::io::SeekFrom::Start(0)).expect("Unable to seek precompiled file during precompilation");
                    precompile_file.read_to_string(&mut precompile_file_data).expect("Unable to read precompiled file during precompilation");
                }
            }

            let include_str_regex = Regex::new(Expression::INCLUDE_STR_RGX).unwrap();
            let include_data_regex = Regex::new(Expression::INCLUDE_DATA_RGX).unwrap();
            let mut included_og_paths: Vec<String> = vec![];

            directory.split(":")
                .flat_map(|directory| Path::new(directory).absolutize().ok())
                .filter(|directory| directory.exists() && directory.is_dir())
                .for_each(|directory| {
                    glob(&format!("{}/**/*.swift", directory.display()))
                        .unwrap()
                        .for_each(|entry| {
                            let entry = entry.expect("Unable to crawl entry file");
                            let entry_path = entry.absolutize().expect("Unable to absolutise entry path");
                            let entry_content = std::fs::read(&entry_path).unwrap();
                            let entry_content_str = String::from_utf8(entry_content).unwrap();

                            include_str_regex
                                .captures_iter(entry_content_str.as_str())
                                .into_iter()
                                .chain(include_data_regex.captures_iter(entry_content_str.as_str()))
                                .flatten()
                                .for_each(|capture| {
                                    let include_str_call = capture.get(0).expect("Unable to get include_str call");
                                    let include_str_og_path = capture.get(1).expect("Unable to get include_str path");
                                    let line_number = entry_content_str[..include_str_call.start()]
                                        .matches('\n')
                                        .count()
                                        + 1;

                                    let mut include_str_og_path_str = include_str_og_path
                                        .as_str()
                                        .to_string();
                                    if let Some(path_aliases) = &precompiled_config.path_aliases {
                                        path_aliases.keys()
                                            .into_iter()
                                            .zip(path_aliases.values())
                                            .map(|(alias, path)| (alias, path.as_str()))
                                            .filter(|(_, path)| path.is_some())
                                            .map(|(alias, path)| (alias, path.unwrap()))
                                            .map(|(alias, path)| (alias, PathBuf::from(path)))
                                            .map(|(alias, path)| (alias, if path.is_absolute() {
                                                path.absolutize().unwrap().to_path_buf()
                                            } else {
                                                PathBuf::from(directory.to_owned()).join(path).absolutize().unwrap().to_path_buf()
                                            }))
                                            .filter(|(_, path)| path.exists())
                                            .map(|(alias, path)| (alias, path.display().to_string()))
                                            .for_each(|(alias, path)| {
                                                include_str_og_path_str = include_str_og_path_str.replace(alias, &*path);
                                            });
                                    }

                                    let include_str_path = Path::new(&include_str_og_path_str);
                                    let include_str_path = if include_str_path.is_absolute() {
                                        PathBuf::from(include_str_og_path_str.to_owned())
                                            .absolutize()
                                            .map(|path| path.to_path_buf())
                                    } else {
                                        PathBuf::from(if let Some(entry_parent) = entry.parent() { entry_parent } else { &entry })
                                            .join(include_str_og_path_str.to_owned())
                                            .absolutize()
                                            .map(|path| path.to_path_buf())
                                    };

                                    if let Ok(ref include_str_path) = include_str_path && !include_str_path.exists() {
                                        if xcode_script_renderer {
                                            eprintln!("{}:{}: error : precompiledIncludeStr call references non-existent file {}", entry_path.display(), line_number, include_str_path.display());
                                        } else {
                                            eprintln!("{} precompileIncludeStr call at line {} in {} references non-existent file {}", "Error:".red(), line_number, entry_path.display(), include_str_path.display());
                                        }
                                        exit(1);
                                    }

                                    if !included_og_paths.contains(&include_str_og_path.as_str().to_string()) {
                                        if !dry_run {
                                            let content_of_file = std::fs::read_to_string(include_str_path.as_ref().unwrap()).expect("Unable to read file to embed");
                                            vec!["precompile-content-str", "precompile-content-data"]
                                                .iter()
                                                .for_each(|placeholder| {
                                                    precompile_file_data = precompile_file_data.replace(&*format!("// <{}>", placeholder), &*format!("\
        // <{}>
        case \"{}\":
            content = \"{}\"\n", placeholder, include_str_og_path.as_str(), BASE64_STANDARD.encode(content_of_file.to_owned())));
                                                });
                                        }

                                        included_og_paths.push(include_str_og_path.as_str().to_owned());
                                    }
                                });
                        });
                });

            if let Some(precompile_file) = precompile_file.as_mut() {
                precompile_file.seek(std::io::SeekFrom::Start(0)).expect("Unable to seek precompiled file during precompilation");
                precompile_file.write_all(precompile_file_data.as_bytes()).expect("Unable to write precompiled file during precompilation");
            }

            let precompile_time_millis = precompile_time.elapsed().as_millis();
            println!("{}: Precompiled {} calls in {}{}", "success".green(), included_og_paths.len(),
                     (if precompile_time_millis < 1000 { format!("{}ms", precompile_time_millis) } else { format!("{}s", precompile_time.elapsed().as_secs()) }).to_string().bold(),
                     if dry_run || xcode_script_renderer { String::new() } else { format!(", add {} to your Xcode build phase", out_abs_path.bold()) }
            );
        }
        Commands::Clean {
            out
        } => {
            let out_path = Path::new(&out);
            let out_abs_path = out_path
                .absolutize()
                .unwrap()
                .display()
                .to_string()
                .to_owned();
            if out_path.is_dir() {
                println!("{} already exists as a directory", out_abs_path);
                return;
            }

            if !out_path.exists() {
                eprintln!("{} precompiled file {} does not exist", "error:".red(), out_abs_path);
                exit(1);
            }

            std::fs::remove_file(&out_path).expect("Unable to delete precompiled file");
        },
        Commands::Init {
            config
        } => {
            let config_path = Path::new(&config).absolutize().expect("Unable to absolutize config path");
            if config_path.exists() {
                println!("{}: Config file already exists at {}", "error".red(), config_path.display().to_string().bold());
                exit(1);
            }
            let mut config_file = File::create(&config).expect("Unable to create config file");

            toml::to_string_pretty(&Config::default()).expect("Unable to serialize default config")
                .lines()
                .for_each(|line| {
                    config_file.write_all(format!("{}\n", line).as_bytes()).expect("Unable to write config file");
                });

            println!("{}: Created config file at {}", "success".green(), config_path.display().to_string().bold());
        }
    }
}
