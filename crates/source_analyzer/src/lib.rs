use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};
use syn::{File, Ident, Item, Type};

#[derive(Serialize, Deserialize, Debug)]
pub struct CyclerConfiguration {
    pub name: String,
    pub kind: CyclerKind,
    pub instances: Option<Vec<String>>,
    pub module: String,
    pub nodes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CyclerKind {
    Perception,
    RealTime,
}

type CyclerName = String;
type NodeName = String;

#[derive(Debug)]
pub struct Cyclers {
    pub cyclers: BTreeMap<CyclerName, Cycler>,
}

#[derive(Debug)]
pub struct Cycler {
    pub kind: CyclerKind,
    pub instances: Vec<String>,
    pub module: String,
    pub nodes: BTreeMap<NodeName, Node>,
}

#[derive(Debug)]
pub struct Node {
    pub creation_context: Vec<Field>,
    pub cycle_context: Vec<Field>,
    pub main_outputs: Vec<Field>,
}

impl Node {
    pub fn try_from_file(file_path: impl AsRef<Path>, file: &File) -> Result<Self> {
        let uses = uses_from_items(&file.items);
        let mut creation_context = vec![];
        let mut cycle_context = vec![];
        let mut main_outputs = vec![];
        for item in file.items.iter() {
            match item {
                Item::Struct(struct_item)
                    if struct_item.attrs.iter().any(|attribute| {
                        attribute
                            .path
                            .get_ident()
                            .map(|attribute_name| attribute_name == "context")
                            .unwrap_or(false)
                    }) =>
                {
                    let mut fields = struct_item
                        .fields
                        .iter()
                        .map(|field| Field::try_from_field(&file_path, field, &uses))
                        .collect::<Result<_, _>>()
                        .wrap_err("failed to gather context fields")?;
                    match struct_item.ident.to_string().as_str() {
                        "CreationContext" => {
                            creation_context.append(&mut fields);
                        }
                        "CycleContext" => {
                            cycle_context.append(&mut fields);
                        }
                        "MainOutputs" => {
                            main_outputs.append(&mut fields);
                        }
                        _ => {
                            return new_syn_error_as_eyre_result(
                                struct_item.ident.span(),
                                "expected `CreationContext`, `CycleContext`, or `MainOutputs`",
                                file_path,
                            );
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            creation_context,
            cycle_context,
            main_outputs,
        })
    }
}

#[derive(Debug)]
pub enum Field {
    AdditionalOutput {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    CyclerInstance {
        name: Ident,
    },
    HardwareInterface {
        name: Ident,
    },
    HistoricInput {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    Input {
        cycler_instance: Option<String>,
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    MainOutput {
        data_type: Type,
        name: Ident,
    },
    Parameter {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    PerceptionInput {
        cycler_instance: String,
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    PersistentState {
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
    RequiredInput {
        cycler_instance: Option<String>,
        data_type: Type,
        name: Ident,
        path: Vec<PathSegment>,
    },
}

impl Field {
    pub fn try_from_field(
        file_path: impl AsRef<Path>,
        field: &syn::Field,
        uses: &Uses,
    ) -> Result<Self> {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| eyre!("field must have be named"))?;
        match &field.ty {
            Type::Path(path) => {
                if path.path.segments.len() != 1 {
                    return new_syn_error_as_eyre_result(
                        path.span(),
                        "expected type path with exactly one segment",
                        file_path,
                    );
                }
                let first_segment = &path.path.segments[0];
                match first_segment.ident.to_string().as_str() {
                    "AdditionalOutput" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        let path_contains_optional = path.iter().any(|segment| segment.is_optional);
                        if path_contains_optional {
                            bail!("unexpected optional segments in path of additional output `{field_name}`");
                        }
                        Ok(Field::AdditionalOutput {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "CyclerInstance" => Ok(Field::CyclerInstance {
                        name: field_name.clone(),
                    }),
                    "HardwareInterface" => Ok(Field::HardwareInterface {
                        name: field_name.clone(),
                    }),
                    "HistoricInput" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::HistoricInput {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "Input" => {
                        let (data_type, cycler_instance, path) = match &first_segment.arguments {
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 2 =>
                            {
                                let (data_type, path) =
                                    extract_two_arguments(file_path, &first_segment.arguments)?;
                                (data_type, None, path)
                            }
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 3 =>
                            {
                                let (data_type, cycler_instance, path) =
                                    extract_three_arguments(file_path, &first_segment.arguments)?;
                                (data_type, Some(cycler_instance), path)
                            }
                            _ => new_syn_error_as_eyre_result(
                                first_segment.arguments.span(),
                                "expected exactly two or three generic parameters",
                                file_path,
                            )?,
                        };
                        Ok(Field::Input {
                            cycler_instance,
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "MainOutput" => {
                        let data_type = extract_one_argument(file_path, &first_segment.arguments)?;
                        Ok(Field::MainOutput {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                        })
                    }
                    "Parameter" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::Parameter {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "PerceptionInput" => {
                        let (data_type, cycler_instance, path) =
                            extract_three_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::PerceptionInput {
                            cycler_instance,
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "PersistentState" => {
                        let (data_type, path) =
                            extract_two_arguments(file_path, &first_segment.arguments)?;
                        Ok(Field::PersistentState {
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    "RequiredInput" => {
                        let (data_type, cycler_instance, path) = match &first_segment.arguments {
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 2 =>
                            {
                                let (data_type, path) =
                                    extract_two_arguments(file_path, &first_segment.arguments)?;
                                (data_type, None, path)
                            }
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 3 =>
                            {
                                let (data_type, cycler_instance, path) =
                                    extract_three_arguments(file_path, &first_segment.arguments)?;
                                (data_type, Some(cycler_instance), path)
                            }
                            _ => new_syn_error_as_eyre_result(
                                first_segment.arguments.span(),
                                "expected exactly two or three generic parameters",
                                file_path,
                            )?,
                        };
                        let path_contains_optional = path.iter().any(|segment| segment.is_optional);
                        if !path_contains_optional {
                            bail!("expected optional segments in path of required input `{field_name}`");
                        }
                        Ok(Field::RequiredInput {
                            cycler_instance,
                            data_type: data_type.to_absolute(uses),
                            name: field_name.clone(),
                            path,
                        })
                    }
                    _ => new_syn_error_as_eyre_result(
                        first_segment.ident.span(),
                        "unexpected identifier",
                        file_path,
                    ),
                }
            }
            _ => new_syn_error_as_eyre_result(field.ty.span(), "expected type path", file_path),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PathSegment {
    pub name: String,
    pub is_optional: bool,
    pub is_variable: bool,
}

impl From<&str> for PathSegment {
    fn from(segment: &str) -> Self {
        let (is_variable, start_index) = match segment.starts_with('$') {
            true => (true, 1),
            false => (false, 0),
        };
        let (is_optional, end_index) = match segment.ends_with('?') {
            true => (true, segment.chars().count() - 1),
            false => (false, segment.chars().count()),
        };

        Self {
            name: segment[start_index..end_index].to_string(),
            is_optional,
            is_variable,
        }
    }
}

// mod configuration;
// mod contexts;
// mod cycler_crates;
// mod cycler_instances;
// mod cycler_types;
// mod into_eyre_result;
// mod nodes;
// mod parse;
// mod structs;
// mod to_absolute;
// mod uses;

// pub use configuration::{CyclerConfiguration, CyclerKind};
// pub use contexts::{expand_variables_from_path, Contexts, Field, PathSegment};
// pub use cycler_instances::CyclerInstances;
// pub use cycler_types::{CyclerType, CyclerTypes};
// pub use nodes::{Node, Nodes};
// pub use parse::parse_rust_file;
// pub use structs::{CyclerStructs, StructHierarchy, Structs};
