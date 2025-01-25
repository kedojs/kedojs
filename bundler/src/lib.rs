use std::{collections::HashMap, fs, path::PathBuf, time::Duration};

use anyhow::Error;
use swc_bundler::{
    Bundle, Bundler, Config, Hook, Load, ModuleData, ModuleRecord, Resolve,
};
use swc_common::{
    comments::SingleThreadedComments,
    errors::{ColorConfig, Handler},
    sync::Lrc,
    FileName, Globals, Mark, SourceFile, SourceMap, Span, GLOBALS,
};
use swc_ecma_ast::{EsVersion, Expr, Ident, KeyValueProp, Lit, Program, PropName, Str};
use swc_ecma_codegen::{
    text_writer::{omit_trailing_semi, JsWriter, WriteJs},
    to_code_default, Emitter,
};
use swc_ecma_loader::{resolve::Resolution, resolvers::lru::CachingResolver};
// use swc_ecma_minifier::option::{
//     CompressOptions, ExtraOptions, MinifyOptions, TopLevelOptions,
// };
use swc_ecma_parser::{
    lexer::Lexer, parse_file_as_module, Parser, StringInput, Syntax, TsSyntax,
};
use swc_ecma_transforms_base::{fixer::fixer, hygiene::hygiene, resolver};
use swc_ecma_transforms_typescript::strip;
use swc_ecma_visit::{FoldWith, VisitMutWith};

pub struct KedoHook;

impl Hook for KedoHook {
    fn get_import_meta_props(
        &self,
        span: Span,
        module_record: &ModuleRecord,
    ) -> Result<Vec<KeyValueProp>, Error> {
        let file_url = module_record.file_name.to_string();

        let dir = file_url
            .rsplitn(2, '/')
            .last()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let file = file_url
            .rsplitn(2, '/')
            .next()
            .map(|s| s.to_string())
            .unwrap_or_default();

        Ok(vec![
            KeyValueProp {
                key: PropName::Ident(Ident::new("url".into(), span)),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                    span,
                    raw: None,
                    value: file_url.into(),
                }))),
            },
            KeyValueProp {
                key: PropName::Ident(Ident::new("dir".into(), span)),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                    span,
                    raw: None,
                    value: dir.into(),
                }))),
            },
            KeyValueProp {
                key: PropName::Ident(Ident::new("filename".into(), span)),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                    span,
                    raw: None,
                    value: file.into(),
                }))),
            },
        ])
    }
}

struct PathLoader {
    cm: Lrc<SourceMap>,
}

impl Load for PathLoader {
    fn load(&self, file: &FileName) -> Result<ModuleData, Error> {
        let file = match file {
            FileName::Real(v) => v,
            _ => unreachable!(),
        };

        let fm = self.cm.load_file(file)?;

        let module = parse_file_as_module(
            &fm,
            Syntax::Typescript(TsSyntax {
                tsx: false,
                decorators: false,
                dts: false,
                no_early_errors: false,
                ..Default::default()
            }),
            EsVersion::Es2022,
            None,
            &mut vec![],
        )
        .expect("This should not happen");

        Ok(ModuleData {
            fm,
            module,
            helpers: Default::default(),
        })
    }
}

struct PathResolver;

impl Resolve for PathResolver {
    fn resolve(
        &self,
        base: &FileName,
        module_specifier: &str,
    ) -> Result<Resolution, Error> {
        assert!(
            module_specifier.starts_with('.'),
            "{}",
            format!(
                "Module specifier should start with '.': {}",
                module_specifier.to_string()
            )
        );

        let base = match base {
            FileName::Real(v) => v,
            _ => unreachable!(),
        };

        Ok(Resolution {
            filename: FileName::Real(
                base.parent()
                    .unwrap()
                    .join(module_specifier)
                    .with_extension("ts"),
            ),
            slug: None,
        })
    }
}

pub struct BundleArgs {
    pub external_modules: Vec<String>,
    pub entries: Vec<(String, PathBuf)>,
    pub outputs: Vec<PathBuf>,
    pub minify: bool,
}

pub struct BundleResult {
    pub duration: Duration,
}

pub fn bundle(args: BundleArgs) -> Result<BundleResult, Error> {
    let cm = Lrc::new(SourceMap::default());
    let globals = Globals::new();

    let loader = PathLoader { cm: cm.clone() };
    let path_resolver = PathResolver;

    let external_modules = args
        .external_modules
        .into_iter()
        .map(|v| v.into())
        .collect::<Vec<_>>();

    let mut bundler = Bundler::new(
        &globals,
        cm.clone(),
        loader,
        CachingResolver::new(4096, path_resolver),
        Config {
            require: false,
            external_modules,
            disable_dce: true,
            ..Default::default()
        },
        Box::new(KedoHook),
    );

    let mut entries = HashMap::default();

    let start = std::time::Instant::now();

    for (path, entry) in args.entries.iter() {
        entries.insert(path.clone(), entry.clone().into());
    }

    // entries.insert("main".into(), args.entries[0].1.clone().into());
    let mut modules = bundler.bundle(entries)?;
    // let c = swc::Compiler::new(cm.clone());

    if args.minify {
        modules = modules
            .into_iter()
            .map(|mut b| {
                GLOBALS.set(&globals, || {
                    let unresolved_mark = Mark::new();
                    let top_level_mark = Mark::new();
                    // Conduct identifier scope analysis
                    b.module = b.module.fold_with(&mut resolver(
                        unresolved_mark,
                        top_level_mark,
                        true,
                    ));

                    b.module = b.module.fold_with(&mut hygiene());
                    // Remove typescript types
                    let program: Program = b.module.into();
                    b.module = program
                        .fold_with(&mut strip(top_level_mark))
                        .expect_module();
                    // b.module.visit_mut_with(&mut strip(top_level_mark));
                    // b.module = swc_ecma_minifier::optimize(
                    //     b.module.into(),
                    //     cm.clone(),
                    //     None,
                    //     None,
                    //     &MinifyOptions {
                    //         compress: Some(CompressOptions {
                    //             top_level: Some(TopLevelOptions { functions: true }),
                    //             keep_classnames: true,
                    //             keep_fargs: true,
                    //             ..Default::default()
                    //         }),
                    //         mangle: None,
                    //         // Some(MangleOptions {
                    //         //     keep_class_names: true,
                    //         //     safari10: true,
                    //         //     top_level: Some(true),
                    //         //     keep_private_props: true,
                    //         //     ..Default::default()
                    //         // }),
                    //         ..Default::default()
                    //     },
                    //     &ExtraOptions {
                    //         unresolved_mark,
                    //         top_level_mark,
                    //     },
                    // )
                    // .expect_module();
                    b.module.visit_mut_with(&mut fixer(None));
                    b
                })
            })
            .collect();
    }

    write_output(&args.outputs, cm, modules, args.minify);

    Ok(BundleResult {
        duration: start.elapsed(),
    })
}

fn write_output(
    outputs: &Vec<PathBuf>,
    cm: Lrc<SourceMap>,
    modules: Vec<Bundle>,
    minify: bool,
) {
    let mut index = 0;
    for bundled in modules {
        let code = {
            let mut buf = vec![];

            {
                let wr = JsWriter::new(cm.clone(), "\n", &mut buf, None);
                let mut emitter = Emitter {
                    cfg: swc_ecma_codegen::Config::default().with_minify(minify),
                    cm: cm.clone(),
                    comments: None,
                    wr: if minify {
                        Box::new(omit_trailing_semi(wr)) as Box<dyn WriteJs>
                    } else {
                        Box::new(wr) as Box<dyn WriteJs>
                    },
                };

                emitter.emit_module(&bundled.module).unwrap();
            }

            String::from_utf8_lossy(&buf).to_string()
        };

        let output_path = &outputs[index];
        println!(
            "Created {} ({}kb)",
            output_path.display(),
            code.len() / 1024
        );

        validate_file_dir(output_path).unwrap();
        fs::write(output_path, &code).unwrap();
        index += 1;
    }
}

// if the dir path of a given file does not exist, it must be created
pub fn validate_file_dir(path: &PathBuf) -> Result<(), String> {
    let dir = path.parent().unwrap();
    if !dir.exists() {
        fs::create_dir_all(dir).unwrap();
    }
    Ok(())
}

pub fn compile_typescript(fm: &SourceFile, cm: Lrc<SourceMap>) {
    let handler =
        Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
    let comments = SingleThreadedComments::default();

    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: false,
            decorators: true,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);

    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let module = parser
        .parse_program()
        .map_err(|e| e.into_diagnostic(&handler).emit())
        .expect("failed to parse module.");

    let globals = Globals::default();
    GLOBALS.set(&globals, || {
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();

        // Conduct identifier scope analysis
        let module =
            module.fold_with(&mut resolver(unresolved_mark, top_level_mark, true));

        // Optionally transforms decorators here before the resolver pass
        // as it might produce runtime declarations.
        // let module = module.fold_with(&mut fold_decorators());

        // Remove typescript types
        let module = module.fold_with(&mut strip(top_level_mark));

        // Fix up any identifiers with the same name, but different contexts
        let module = module.fold_with(&mut hygiene());

        // Ensure that we have enough parenthesis.
        let program = module.fold_with(&mut fixer(Some(&comments)));

        println!("{}", to_code_default(cm, Some(&comments), &program));
    })
}
