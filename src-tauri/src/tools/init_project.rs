use std::fs;

use serde_json::Value;

use super::ToolContext;

#[derive(Clone, Copy)]
enum ProjectLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    CSharp,
    Cpp,
    Php,
    Ruby,
}

impl ProjectLanguage {
    fn parse(raw: &str) -> Result<Self, String> {
        let key = raw.trim().to_ascii_lowercase();
        match key.as_str() {
            "rust" | "rs" => Ok(Self::Rust),
            "typescript" | "ts" | "tsx" => Ok(Self::TypeScript),
            "javascript" | "js" | "nodejs" | "node" => Ok(Self::JavaScript),
            "python" | "py" => Ok(Self::Python),
            "go" | "golang" => Ok(Self::Go),
            "java" => Ok(Self::Java),
            "csharp" | "c#" | "cs" | "dotnet" => Ok(Self::CSharp),
            "cpp" | "c++" | "cxx" => Ok(Self::Cpp),
            "php" => Ok(Self::Php),
            "ruby" | "rb" => Ok(Self::Ruby),
            other => Err(format!(
                "unsupported language '{other}'. Supported: rust, typescript, javascript, python, go, java, csharp, cpp, php, ruby"
            )),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::TypeScript => "TypeScript",
            Self::JavaScript => "JavaScript",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::CSharp => "C#",
            Self::Cpp => "C++",
            Self::Php => "PHP",
            Self::Ruby => "Ruby",
        }
    }
}

fn sanitize_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("project name cannot be empty".into());
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("project name must be a single path segment".into());
    }
    let ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(
            "project name may only contain letters, numbers, '-' and '_'".into(),
        );
    }
    Ok(name.to_string())
}

fn rust_crate_name(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if i > 0 && !out.ends_with('_') {
            out.push('_');
        }
    }
    if out.is_empty() {
        "app".into()
    } else if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("app_{out}")
    } else {
        out
    }
}

/// Initialize an idiomatic language project under the workspace and write starter files.
pub fn init_project(ctx: &ToolContext<'_>, args: &Value) -> Result<String, String> {
    let language_raw = args
        .get("language")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing language".to_string())?;
    let language = ProjectLanguage::parse(language_raw)?;

    let name = sanitize_name(
        args.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("app"),
    )?;

    let directory = args
        .get("directory")
        .and_then(|v| v.as_str())
        .unwrap_or(".")
        .trim();

    let relative_root = if directory.is_empty() || directory == "." {
        name.clone()
    } else {
        let trimmed = directory.trim_matches('/').trim_matches('\\');
        format!("{trimmed}/{name}")
    };

    let project_root = ctx
        .sandbox
        .resolve(&relative_root)
        .map_err(|e| e.to_string())?;

    if project_root.exists() {
        let has_entries = fs::read_dir(&project_root)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        if has_entries {
            return Err(format!(
                "directory already exists and is not empty: {relative_root}. Choose another name or edit the existing project."
            ));
        }
    }

    fs::create_dir_all(&project_root).map_err(|e| e.to_string())?;

    let files = scaffold_files(language, &name);
    let mut created = Vec::new();

    for (rel, content) in files {
        let path = project_root.join(&rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // Keep scaffolding inside workspace
        ctx.sandbox
            .ensure_within(&path)
            .map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())?;
        created.push(format!("{relative_root}/{rel}").replace('\\', "/"));
    }

    Ok(format!(
        "Initialized {} project '{name}' at {relative_root}.\nCreated files:\n{}",
        language.label(),
        created
            .iter()
            .map(|p| format!("- {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

fn scaffold_files(language: ProjectLanguage, name: &str) -> Vec<(String, String)> {
    match language {
        ProjectLanguage::Rust => {
            let crate_name = rust_crate_name(name);
            vec![
                (
                    "Cargo.toml".into(),
                    format!(
                        r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
                    ),
                ),
                (
                    "src/main.rs".into(),
                    r#"fn main() {
    println!("Hello, world!");
}
"#
                    .into(),
                ),
                (
                    ".gitignore".into(),
                    "/target\nCargo.lock\n".into(),
                ),
                (
                    "README.md".into(),
                    format!("# {name}\n\nRust project generated by Desktop Agent.\n\n```bash\ncargo run\n```\n"),
                ),
            ]
        }
        ProjectLanguage::TypeScript => vec![
            (
                "package.json".into(),
                format!(
                    r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {{
    "dev": "tsx src/index.ts",
    "build": "tsc -p tsconfig.json",
    "start": "node dist/index.js"
  }},
  "devDependencies": {{
    "tsx": "^4.19.0",
    "typescript": "^5.8.0",
    "@types/node": "^22.0.0"
  }}
}}
"#
                ),
            ),
            (
                "tsconfig.json".into(),
                r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src/**/*"]
}
"#
                .into(),
            ),
            (
                "src/index.ts".into(),
                r#"function main(): void {
  console.log("Hello, world!");
}

main();
"#
                .into(),
            ),
            (
                ".gitignore".into(),
                "node_modules/\ndist/\n*.log\n".into(),
            ),
            (
                "README.md".into(),
                format!("# {name}\n\nTypeScript project generated by Desktop Agent.\n\n```bash\nnpm install\nnpm run dev\n```\n"),
            ),
        ],
        ProjectLanguage::JavaScript => vec![
            (
                "package.json".into(),
                format!(
                    r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {{
    "start": "node src/index.js",
    "dev": "node --watch src/index.js"
  }}
}}
"#
                ),
            ),
            (
                "src/index.js".into(),
                r#"function main() {
  console.log("Hello, world!");
}

main();
"#
                .into(),
            ),
            (
                ".gitignore".into(),
                "node_modules/\n*.log\n".into(),
            ),
            (
                "README.md".into(),
                format!("# {name}\n\nJavaScript project generated by Desktop Agent.\n\n```bash\nnpm start\n```\n"),
            ),
        ],
        ProjectLanguage::Python => {
            let module = rust_crate_name(name); // same sanitization works for module
            vec![
                (
                    "pyproject.toml".into(),
                    format!(
                        r#"[project]
name = "{name}"
version = "0.1.0"
description = "Python project generated by Desktop Agent"
requires-python = ">=3.10"
dependencies = []

[build-system]
requires = ["setuptools>=68"]
build-backend = "setuptools.build_meta"
"#
                    ),
                ),
                (
                    format!("src/{module}/__init__.py"),
                    r#""""Project package."""
"#
                    .into(),
                ),
                (
                    format!("src/{module}/main.py"),
                    r#"def main() -> None:
    print("Hello, world!")


if __name__ == "__main__":
    main()
"#
                    .into(),
                ),
                (
                    "requirements.txt".into(),
                    "# Add runtime dependencies here\n".into(),
                ),
                (
                    ".gitignore".into(),
                    "__pycache__/\n*.py[cod]\n.venv/\nvenv/\ndist/\n*.egg-info/\n".into(),
                ),
                (
                    "README.md".into(),
                    format!("# {name}\n\nPython project generated by Desktop Agent.\n\n```bash\npython -m src.{module}.main\n```\n"),
                ),
            ]
        }
        ProjectLanguage::Go => {
            let module = format!("example.com/{name}");
            vec![
                (
                    "go.mod".into(),
                    format!("module {module}\n\ngo 1.22\n"),
                ),
                (
                    "main.go".into(),
                    r#"package main

import "fmt"

func main() {
	fmt.Println("Hello, world!")
}
"#
                    .into(),
                ),
                (
                    ".gitignore".into(),
                    "*.exe\n*.test\n/bin/\n".into(),
                ),
                (
                    "README.md".into(),
                    format!("# {name}\n\nGo project generated by Desktop Agent.\n\n```bash\ngo run .\n```\n"),
                ),
            ]
        }
        ProjectLanguage::Java => {
            let package_path = "com/example/app";
            vec![
                (
                    "pom.xml".into(),
                    format!(
                        r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>{name}</artifactId>
  <version>0.1.0</version>
  <properties>
    <maven.compiler.release>17</maven.compiler.release>
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
  </properties>
</project>
"#
                    ),
                ),
                (
                    format!("src/main/java/{package_path}/Main.java"),
                    r#"package com.example.app;

public final class Main {
    private Main() {}

    public static void main(String[] args) {
        System.out.println("Hello, world!");
    }
}
"#
                    .into(),
                ),
                (
                    ".gitignore".into(),
                    "target/\n*.class\n.idea/\n*.iml\n".into(),
                ),
                (
                    "README.md".into(),
                    format!("# {name}\n\nJava (Maven) project generated by Desktop Agent.\n"),
                ),
            ]
        }
        ProjectLanguage::CSharp => vec![
            (
                format!("{name}.csproj"),
                r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>
"#
                .into(),
            ),
            (
                "Program.cs".into(),
                r#"Console.WriteLine("Hello, world!");
"#
                .into(),
            ),
            (
                ".gitignore".into(),
                "bin/\nobj/\n*.user\n".into(),
            ),
            (
                "README.md".into(),
                format!("# {name}\n\nC# project generated by Desktop Agent.\n\n```bash\ndotnet run\n```\n"),
            ),
        ],
        ProjectLanguage::Cpp => vec![
            (
                "CMakeLists.txt".into(),
                format!(
                    r#"cmake_minimum_required(VERSION 3.16)
project({name} LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
add_executable({name} src/main.cpp)
"#
                ),
            ),
            (
                "src/main.cpp".into(),
                r#"#include <iostream>

int main() {
    std::cout << "Hello, world!" << std::endl;
    return 0;
}
"#
                .into(),
            ),
            (
                ".gitignore".into(),
                "build/\ncmake-build-*/\n*.o\n*.out\n".into(),
            ),
            (
                "README.md".into(),
                format!("# {name}\n\nC++ (CMake) project generated by Desktop Agent.\n"),
            ),
        ],
        ProjectLanguage::Php => vec![
            (
                "composer.json".into(),
                format!(
                    r#"{{
  "name": "app/{name}",
  "description": "PHP project generated by Desktop Agent",
  "type": "project",
  "require": {{
    "php": ">=8.1"
  }},
  "autoload": {{
    "psr-4": {{
      "App\\": "src/"
    }}
  }}
}}
"#
                ),
            ),
            (
                "public/index.php".into(),
                r#"<?php

declare(strict_types=1);

require __DIR__ . '/../src/Hello.php';

use App\Hello;

echo (new Hello())->greet(), PHP_EOL;
"#
                .into(),
            ),
            (
                "src/Hello.php".into(),
                r#"<?php

declare(strict_types=1);

namespace App;

final class Hello
{
    public function greet(): string
    {
        return 'Hello, world!';
    }
}
"#
                .into(),
            ),
            (
                ".gitignore".into(),
                "vendor/\n.phpunit.result.cache\n".into(),
            ),
            (
                "README.md".into(),
                format!("# {name}\n\nPHP project generated by Desktop Agent.\n\n```bash\nphp public/index.php\n```\n"),
            ),
        ],
        ProjectLanguage::Ruby => vec![
            (
                "Gemfile".into(),
                r#"# frozen_string_literal: true

source "https://rubygems.org"

ruby ">= 3.1.0"
"#
                .into(),
            ),
            (
                "lib/hello.rb".into(),
                r#"# frozen_string_literal: true

module Hello
  module_function

  def greet
    "Hello, world!"
  end
end
"#
                .into(),
            ),
            (
                "bin/main".into(),
                r#"#!/usr/bin/env ruby
# frozen_string_literal: true

require_relative "../lib/hello"

puts Hello.greet
"#
                .into(),
            ),
            (
                ".gitignore".into(),
                ".bundle/\nvendor/bundle/\n*.gem\n".into(),
            ),
            (
                "README.md".into(),
                format!("# {name}\n\nRuby project generated by Desktop Agent.\n\n```bash\nruby bin/main\n```\n"),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_languages() {
        assert!(matches!(
            ProjectLanguage::parse("ts").unwrap(),
            ProjectLanguage::TypeScript
        ));
        assert!(ProjectLanguage::parse("fortran").is_err());
    }

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_name("my-app").unwrap(), "my-app");
        assert!(sanitize_name("../x").is_err());
    }
}
