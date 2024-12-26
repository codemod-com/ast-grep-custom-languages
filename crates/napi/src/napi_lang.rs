use ast_grep_language::SupportLang;
use ignore::types::{Types, TypesBuilder};
use ignore::{WalkBuilder, WalkParallel};
use napi::anyhow::anyhow;
use napi::anyhow::Error;
use napi::bindgen_prelude::Result;
use napi_derive::napi;

use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

#[napi(string_enum)]
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Lang {
  Angular,
  Html,
  JavaScript,
  Tsx,
  Css,
  TypeScript,
  Bash,
  C,
  Cpp,
  CSharp,
  Go,
  Elixir,
  Haskell,
  Java,
  Json,
  Kotlin,
  Lua,
  Php,
  Python,
  Ruby,
  Rust,
  Scala,
  Sql,
  Swift,
  Yaml,
}

impl From<Lang> for SupportLang {
  fn from(val: Lang) -> Self {
    use Lang as F;
    use SupportLang as S;
    match val {
      F::Angular => S::Angular,
      F::Html => S::Html,
      F::JavaScript => S::JavaScript,
      F::Tsx => S::Tsx,
      F::Css => S::Css,
      F::TypeScript => S::TypeScript,
      F::Bash => S::Bash,
      F::C => S::C,
      F::Cpp => S::Cpp,
      F::CSharp => S::CSharp,
      F::Go => S::Go,
      F::Elixir => S::Elixir,
      F::Haskell => S::Haskell,
      F::Java => S::Java,
      F::Json => S::Json,
      F::Kotlin => S::Kotlin,
      F::Lua => S::Lua,
      F::Php => S::Php,
      F::Python => S::Python,
      F::Ruby => S::Ruby,
      F::Rust => S::Rust,
      F::Scala => S::Scala,
      F::Sql => S::Sql,
      F::Swift => S::Swift,
      F::Yaml => S::Yaml,
    }
  }
}

impl From<SupportLang> for Lang {
  fn from(value: SupportLang) -> Self {
    use Lang as F;
    use SupportLang as S;
    match value {
      S::Angular => F::Angular,
      S::Html => F::Html,
      S::JavaScript => F::JavaScript,
      S::Tsx => F::Tsx,
      S::Css => F::Css,
      S::TypeScript => F::TypeScript,
      S::Bash => F::Bash,
      S::C => F::C,
      S::Cpp => F::Cpp,
      S::CSharp => F::CSharp,
      S::Go => F::Go,
      S::Elixir => F::Elixir,
      S::Haskell => F::Haskell,
      S::Java => F::Java,
      S::Json => F::Json,
      S::Kotlin => F::Kotlin,
      S::Lua => F::Lua,
      S::Php => F::Php,
      S::Python => F::Python,
      S::Ruby => F::Ruby,
      S::Rust => F::Rust,
      S::Scala => F::Scala,
      S::Sql => F::Sql,
      S::Swift => F::Swift,
      S::Yaml => F::Yaml,
    }
  }
}

impl Lang {
  pub fn find_files(
    &self,
    paths: Vec<String>,
    language_globs: Option<Vec<String>>,
  ) -> Result<WalkParallel> {
    find_files_with_lang(self, paths, language_globs)
  }
  pub fn lang_globs(map: HashMap<String, Vec<String>>) -> LanguageGlobs {
    let mut ret = HashMap::new();
    for (name, patterns) in map {
      if let Ok(lang) = Lang::from_str(&name) {
        ret.insert(lang, patterns);
      }
    }
    ret
  }
}

pub type LanguageGlobs = HashMap<Lang, Vec<String>>;

impl FromStr for Lang {
  type Err = Error;
  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    SupportLang::from_str(s)
      .map(|l| l.into())
      .map_err(|_| anyhow!(format!("{s} is not supported in napi")))
  }
}

pub enum LangOption {
  /// Used when language is inferred from file path
  /// e.g. in parse_files
  Inferred(Vec<(SupportLang, Types)>),
  /// Used when language is specified
  /// e.g. in frontend_lang.find_in_files
  Specified(Lang),
}

impl LangOption {
  pub fn get_lang(&self, path: &Path) -> Option<SupportLang> {
    use LangOption::*;
    match self {
      Specified(lang) => Some((*lang).into()),
      Inferred(pairs) => pairs
        .iter()
        .find_map(|(lang, types)| types.matched(path, false).is_whitelist().then_some(*lang)),
    }
  }
  pub fn infer(language_globs: &LanguageGlobs) -> Self {
    let mut types = vec![];
    let empty = vec![];
    for lang in SupportLang::all_langs() {
      let mut builder = TypesBuilder::new();
      let tpe = lang.to_string();
      let file_types = lang.file_types();
      add_types(&mut builder, &file_types);
      let fe_lang = Lang::from(*lang);
      for pattern in language_globs.get(&fe_lang).unwrap_or(&empty) {
        builder.add(&tpe, pattern).expect("should build");
      }
      builder.select(&tpe);
      types.push((*lang, builder.build().unwrap()));
    }
    Self::Inferred(types)
  }
}

pub fn build_files(paths: Vec<String>, language_globs: &LanguageGlobs) -> Result<WalkParallel> {
  if paths.is_empty() {
    return Err(anyhow!("paths cannot be empty.").into());
  }
  let mut types = TypesBuilder::new();
  let empty = vec![];
  for lang in SupportLang::all_langs() {
    let type_name = lang.to_string();
    let l = Lang::from(*lang);
    let custom = language_globs.get(&l).unwrap_or(&empty);
    let default_types = lang.file_types();
    select_custom(&mut types, &type_name, &default_types, custom);
  }
  let types = types.build().unwrap();
  let mut paths = paths.into_iter();
  let mut builder = WalkBuilder::new(paths.next().unwrap());
  for path in paths {
    builder.add(path);
  }
  let walk = builder.types(types).build_parallel();
  Ok(walk)
}

fn add_types(builder: &mut TypesBuilder, types: &Types) {
  for def in types.definitions() {
    let name = def.name();
    for glob in def.globs() {
      builder.add(name, glob).expect(name);
    }
  }
}

fn select_custom<'b>(
  builder: &'b mut TypesBuilder,
  file_type: &str,
  default_types: &Types,
  custom_suffix_list: &[String],
) -> &'b mut TypesBuilder {
  add_types(builder, default_types);
  for suffix in custom_suffix_list {
    builder
      .add(file_type, suffix)
      .expect("file pattern must compile");
  }
  builder.select(file_type)
}

fn find_files_with_lang(
  lang: &Lang,
  paths: Vec<String>,
  language_globs: Option<Vec<String>>,
) -> Result<WalkParallel> {
  if paths.is_empty() {
    return Err(anyhow!("paths cannot be empty.").into());
  }

  let mut types = TypesBuilder::new();
  let sg_lang: SupportLang = (*lang).into();
  let type_name = sg_lang.to_string();
  let custom_file_type = language_globs.unwrap_or_default();
  let default_types = sg_lang.file_types();
  let types = select_custom(&mut types, &type_name, &default_types, &custom_file_type)
    .build()
    .unwrap();
  let mut paths = paths.into_iter();
  let mut builder = WalkBuilder::new(paths.next().unwrap());
  for path in paths {
    builder.add(path);
  }
  let walk = builder.types(types).build_parallel();
  Ok(walk)
}

#[cfg(test)]
mod test {
  use super::*;

  fn lang_globs() -> HashMap<Lang, Vec<String>> {
    let mut lang = HashMap::new();
    lang.insert("html".into(), vec!["*.vue".into()]);
    Lang::lang_globs(lang)
  }

  #[test]
  fn test_lang_globs() {
    let globs = lang_globs();
    assert!(globs.contains_key(&Lang::Html));
    assert!(!globs.contains_key(&Lang::Tsx));
    assert_eq!(globs[&Lang::Html], vec!["*.vue"]);
  }

  #[test]
  fn test_lang_option() {
    let globs = lang_globs();
    let option = LangOption::infer(&globs);
    let lang = option.get_lang(Path::new("test.vue"));
    assert_eq!(lang, Some(SupportLang::Html));
    let lang = option.get_lang(Path::new("test.html"));
    assert_eq!(lang, Some(SupportLang::Html));
    let lang = option.get_lang(Path::new("test.js"));
    assert_eq!(lang, Some(SupportLang::JavaScript));
    let lang = option.get_lang(Path::new("test.xss"));
    assert_eq!(lang, None);
  }

  #[test]
  fn test_from_str() {
    let lang = Lang::from_str("html");
    assert_eq!(lang.unwrap(), Lang::Html);
    let lang = Lang::from_str("Html");
    assert_eq!(lang.unwrap(), Lang::Html);
    let lang = Lang::from_str("htML");
    assert_eq!(lang.unwrap(), Lang::Html);
    let lang = Lang::from_str("ocaml");
    assert!(lang.is_err());
  }
}
