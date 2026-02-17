use std::path::Path;

use super::expand_cwd;
use super::validation::{TemplateEntry, TemplatesFile, validate_and_convert};

const BUILTIN: &[&str] = &["test", "init", "create-command"];

#[test]
fn expand_cwd_replaces_placeholder() {
    let cwd = Path::new("/home/user/project");
    let out = expand_cwd("CWD: {cwd}", cwd);
    assert_eq!(out, "CWD: /home/user/project");
}

#[test]
fn expand_cwd_preserves_without_placeholder() {
    let cwd = Path::new("/home");
    let out = expand_cwd("no placeholder", cwd);
    assert_eq!(out, "no placeholder");
}

#[test]
fn validate_rejects_duplicate_names() {
    let file = TemplatesFile {
        templates: vec![
            TemplateEntry {
                name: "a".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            },
            TemplateEntry {
                name: "a".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            },
        ],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("Duplicate"));
}

#[test]
fn validate_rejects_builtin_collision() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "test".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("built-in"));
}

#[test]
fn validate_accepts_valid_custom() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "security".to_string(),
            description: "Audit".to_string(),
            prompt_prefix: "Check {cwd}".to_string(),
            mode: "Build".to_string(),
        }],
    };
    let out = validate_and_convert(file, BUILTIN).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "security");
}

#[test]
fn validate_rejects_empty_name() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("cannot be empty"));
}

#[test]
fn validate_rejects_name_with_spaces() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "my command".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(
        err.to_string().contains("letters") || err.to_string().contains("hyphens"),
        "expected validation message about allowed chars, got: {}",
        err
    );
}

#[test]
fn validate_accepts_name_with_hyphens() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "my-command".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let out = validate_and_convert(file, BUILTIN).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "my-command");
}

#[test]
fn validate_accepts_name_with_underscores() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "my_command".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let out = validate_and_convert(file, BUILTIN).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "my_command");
}

#[test]
fn validate_rejects_name_with_special_chars() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "cmd!".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(
        err.to_string().contains("letters") || err.to_string().contains("hyphens"),
        "expected validation message about allowed chars, got: {}",
        err
    );
}

#[test]
fn validate_rejects_builtin_collision_case_insensitive() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "TEST".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("built-in"));
}

#[test]
fn validate_rejects_duplicate_names_case_insensitive() {
    let file = TemplatesFile {
        templates: vec![
            TemplateEntry {
                name: "Foo".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            },
            TemplateEntry {
                name: "foo".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            },
        ],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("Duplicate"));
}

#[test]
fn validate_rejects_invalid_mode() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "custom".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Random".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("mode"));
    assert!(err.to_string().contains("Ask") || err.to_string().contains("Build"));
}

#[test]
fn validate_rejects_mode_lowercase() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "custom".to_string(),
            description: "x".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("mode"));
}

#[test]
fn validate_rejects_empty_description() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "custom".to_string(),
            description: "".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("description"));
}

#[test]
fn validate_rejects_whitespace_only_description() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "custom".to_string(),
            description: "   \t  ".to_string(),
            prompt_prefix: "y".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("description"));
}

#[test]
fn validate_rejects_empty_prompt_prefix() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "custom".to_string(),
            description: "x".to_string(),
            prompt_prefix: "".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("prompt_prefix"));
}

#[test]
fn validate_rejects_whitespace_only_prompt_prefix() {
    let file = TemplatesFile {
        templates: vec![TemplateEntry {
            name: "custom".to_string(),
            description: "x".to_string(),
            prompt_prefix: "\n\t  ".to_string(),
            mode: "Ask".to_string(),
        }],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("prompt_prefix"));
}

#[test]
fn validate_accepts_empty_file() {
    let file = TemplatesFile { templates: vec![] };
    let out = validate_and_convert(file, BUILTIN).unwrap();
    assert!(out.is_empty());
}

#[test]
fn validate_accepts_multiple_valid_templates() {
    let file = TemplatesFile {
        templates: vec![
            TemplateEntry {
                name: "alpha".to_string(),
                description: "First".to_string(),
                prompt_prefix: "Do A".to_string(),
                mode: "Ask".to_string(),
            },
            TemplateEntry {
                name: "beta".to_string(),
                description: "Second".to_string(),
                prompt_prefix: "Do B".to_string(),
                mode: "Build".to_string(),
            },
        ],
    };
    let out = validate_and_convert(file, BUILTIN).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].name, "alpha");
    assert_eq!(out[1].name, "beta");
}

#[test]
fn safe_mode_message_formats_friendly_error() {
    use super::TemplatesError;
    let io_err = TemplatesError::Io(std::io::Error::from(std::io::ErrorKind::NotFound));
    assert!(
        io_err
            .safe_mode_message()
            .contains("built-in commands only")
    );
    assert!(io_err.safe_mode_message().contains("could not read file"));

    let val_err = TemplatesError::Validation("duplicate name".to_string());
    assert!(
        val_err
            .safe_mode_message()
            .contains("built-in commands only")
    );
    assert!(val_err.safe_mode_message().contains("validation error"));
}

#[test]
fn validate_fails_first_invalid_among_many() {
    let file = TemplatesFile {
        templates: vec![
            TemplateEntry {
                name: "valid".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            },
            TemplateEntry {
                name: "".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            },
            TemplateEntry {
                name: "also invalid".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            },
        ],
    };
    let err = validate_and_convert(file, BUILTIN).unwrap_err();
    assert!(err.to_string().contains("index 1") || err.to_string().contains("cannot be empty"));
}
