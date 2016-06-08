#![feature(question_mark)]
extern crate tempdir;
extern crate walkdir;
extern crate yaml_rust;

mod errors;

use std::path::PathBuf;
use std::fs::{self, File};
use std::io::{Read, Write};

use walkdir::{WalkDir};
use yaml_rust::{Yaml, YamlLoader};

pub use errors::SmithyError;


pub struct Smithy<'a> {
    input_path: PathBuf,
    output_path: PathBuf,
    plugins: Vec<Box<SmithyPlugin + 'a>>,
}

impl<'a> Smithy<'a> {
    pub fn builder<P: Into<PathBuf>>(input_path: P, output_path: P) -> Smithy<'a> {
        Smithy {
            input_path: input_path.into(),
            output_path: output_path.into(),
            plugins: vec![],
        }
    }

    pub fn add_plugin<T: SmithyPlugin + 'a>(mut self, plugin: T) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    pub fn build(&mut self) -> Result<(), SmithyError> {
        let mut documents = vec![];
        for entry in WalkDir::new(&self.input_path) {
            let entry = entry?;
            if entry.file_type().is_dir() {
                continue;
            }
            println!("processing {:?}", entry.path());
            let mut input_file = File::open(entry.path())?;
            let mut file_contents = String::new();
            input_file.read_to_string(&mut file_contents)?;

            let rel_path = entry.path().strip_prefix(&self.input_path)?;
            let doc = Document::from_str(rel_path, &file_contents);
            documents.push(doc);
        }

        for plugin in self.plugins.iter() {
            documents = plugin.process(documents)?;
        }

        fs::remove_dir_all(&self.output_path)?;

        for doc in documents {
            let output_file_path = self.output_path.join(doc.path);
            if let Some(parent) = output_file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output_file = File::create(output_file_path)?;
            output_file.write(doc.body.as_bytes())?;
        }
        Ok(())
    }
}

pub trait SmithyPlugin {
    fn process(&self, files: Vec<Document>) -> Result<Vec<Document>, SmithyError> {
        let mut new_files = vec![];
        for file in files.into_iter() {
            new_files.push(self.process_file(file)?);
        }
        Ok(new_files)
    }

    fn process_file(&self, file: Document) -> Result<Document, SmithyError> {
        Ok(file)
    }
}

pub struct Document {
    pub metadata: Yaml,
    pub body: String,
    pub path: PathBuf,
}

impl Document {
    pub fn from_str<T: Into<PathBuf>>(path: T, text: &str) -> Document {
        let path = path.into();
        let splits: Vec<&str> = text.split("---\n").collect();

        if splits.len() >= 3 && splits[0] == "" {
            let front_matter_text = splits[1];
            let body = splits[2..].join("---\n").trim().to_string() + "\n";
            let front_matter = YamlLoader::load_from_str(front_matter_text).unwrap()[0].clone();
            Document { metadata: front_matter, body: body, path: path }
        } else {
            Document { metadata: Yaml::Null, body: text.to_string(), path: path }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::ffi::OsStr;

    use yaml_rust::Yaml;
    use tempdir::TempDir;

    use super::{Document, Smithy, SmithyError, SmithyPlugin};

    #[test]
    fn parse_doc_no_frontmatter() {
        let doc = "Some body";
        let parsed = Document::from_str("doc.txt", doc);
        assert_eq!(parsed.metadata, Yaml::Null);
        assert_eq!(parsed.body, "Some body");
    }

    #[test]
    fn parse_doc_with_frontmatter() {
        let doc = "---
        title: Some doc
        ---

        This is the body of the document.
        ";
        let parsed = Document::from_str("doc.txt", doc);
        match parsed.metadata {
            Yaml::Hash(_) => (),
            _ => {
                println!("Unexpected metadata: {:?}", parsed.metadata);
                assert!(false);
            },
        };
        assert_eq!(parsed.metadata["title"], Yaml::String("Some doc".to_string()));
        assert_eq!(parsed.body, "This is the body of the document.\n");
    }

    #[test]
    fn parse_doc_with_faux_front_matter() {
        let doc = "---\nThis is the body of the document.";
        let parsed = Document::from_str("doc.txt", doc);
        assert_eq!(parsed.metadata, Yaml::Null);
        assert_eq!(parsed.body, "---\nThis is the body of the document.");
    }

    #[test]
    fn test_file_moves() {
        let input_dir = TempDir::new("input").unwrap();
        let output_dir = TempDir::new("output").unwrap();
        let mut input_doc = File::create(input_dir.path().join("doc.txt")).unwrap();
        input_doc.write("---\ntitle: Foo\n---\n\nDocument body".as_bytes()).unwrap();

        Smithy::builder(input_dir.path(), output_dir.path()).build().unwrap();

        let mut output_doc = File::open(output_dir.path().join("doc.txt")).unwrap();
        let mut buf = String::new();
        output_doc.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "Document body\n");
    }

    #[test]
    fn test_plugin_process() {
        let input_dir = TempDir::new("input").unwrap();
        let output_dir = TempDir::new("output").unwrap();
        let mut input_doc = File::create(input_dir.path().join("doc.txt")).unwrap();
        input_doc.write("---\ntitle: Foo\n---\n\nDocument body".as_bytes()).unwrap();

        struct ShoutingPlugin;

        impl SmithyPlugin for ShoutingPlugin {
            fn process(&self, files: Vec<Document>) -> Result<Vec<Document>, SmithyError> {
                Ok(files.into_iter().map(|mut file| {
                    file.body = file.body.to_uppercase();
                    file
                }).collect())
            }
        }

        Smithy::builder(input_dir.path(), output_dir.path())
            .add_plugin(ShoutingPlugin)
            .build()
            .unwrap();

        let mut output_doc = File::open(output_dir.path().join("doc.txt")).unwrap();
        let mut buf = String::new();
        output_doc.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "DOCUMENT BODY\n");
    }

    #[test]
    fn test_plugin_process_file() {
        let input_dir = TempDir::new("input").unwrap();
        let output_dir = TempDir::new("output").unwrap();
        let mut input_doc = File::create(input_dir.path().join("doc.txt")).unwrap();
        input_doc.write("---\ntitle: Foo\n---\n\nDocument body".as_bytes()).unwrap();

        struct ShoutingPlugin;

        impl SmithyPlugin for ShoutingPlugin {
            fn process_file(&self, file: Document) -> Result<Document, SmithyError> {
                let mut file = file;
                file.body = file.body.to_uppercase();
                Ok(file)
            }
        }

        Smithy::builder(input_dir.path(), output_dir.path())
            .add_plugin(ShoutingPlugin)
            .build()
            .unwrap();

        let mut output_doc = File::open(output_dir.path().join("doc.txt")).unwrap();
        let mut buf = String::new();
        output_doc.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "DOCUMENT BODY\n");
    }

    #[test]
    fn test_plugin_move_file() {
        let input_dir = TempDir::new("input").unwrap();
        let output_dir = TempDir::new("output").unwrap();
        let mut input_doc = File::create(input_dir.path().join("foo.txt")).unwrap();
        input_doc.write("---\ntitle: Foo\n---\n\nDocument body".as_bytes()).unwrap();

        struct FooBarPlugin;

        impl SmithyPlugin for FooBarPlugin {
            fn process_file(&self, doc: Document) -> Result<Document, SmithyError> {
                let mut doc = doc;
                if doc.path.file_name() == Some(OsStr::new("foo.txt")) {
                    doc.path = doc.path.with_file_name("bar.txt");
                }
                Ok(doc)
            }
        }

        Smithy::builder(input_dir.path(), output_dir.path())
            .add_plugin(FooBarPlugin)
            .build()
            .unwrap();

        let mut output_doc = File::open(output_dir.path().join("bar.txt")).unwrap();
        let mut buf = String::new();
        output_doc.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "Document body\n");
    }

    #[test]
    fn test_files_in_dirs() {
        let input_dir = TempDir::new("input").unwrap();
        let output_dir = TempDir::new("output").unwrap();

        let foo_dir_path = input_dir.path().join("foo");
        let bar_file_path = foo_dir_path.join("bar.txt");

        fs::create_dir(foo_dir_path).unwrap();
        let mut input_doc = File::create(bar_file_path).unwrap();
        input_doc.write("Document body".as_bytes()).unwrap();

        Smithy::builder(input_dir.path(), output_dir.path()).build().unwrap();

        let output_bar_file_path = output_dir.path().join("foo").join("bar.txt");
        let mut output_doc = File::open(output_bar_file_path).unwrap();
        let mut buf = String::new();
        output_doc.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "Document body");
    }

    #[test]
    fn test_clears_output_dir() {
        let input_dir = TempDir::new("input").unwrap();
        let output_dir = TempDir::new("output").unwrap();

        let foo_file_path = output_dir.path().join("foo.txt");
        File::create(foo_file_path).unwrap();

        Smithy::builder(input_dir.path(), output_dir.path()).build().unwrap();

        let output_foo_file_path = output_dir.path().join("foo.txt");
        assert!(!output_foo_file_path.exists());
    }
}
