use std::{
    collections::{BTreeMap, HashMap},
    env::var,
    fs::{read_to_string, File},
    io::Write,
    path::PathBuf,
    process::Command,
};

use code_generation::{
    cycler::{generate_cyclers, get_cyclers},
    run::generate_run,
};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::{Deserialize, Serialize};
use source_analyzer::{
    CyclerConfiguration, CyclerInstances, CyclerType, CyclerTypes, Field, Nodes, StructHierarchy,
    Structs,
};

#[derive(Serialize, Deserialize, Debug)]
struct FrameworkConfiguration {
    cyclers: Vec<CyclerConfiguration>,
}

fn main() -> Result<()> {
    let configuration: FrameworkConfiguration = toml::from_str(&read_to_string("framework.toml")?)?;
    let cycler_instances = CyclerInstances::from_configuration(&configuration.cyclers);
    dbg!(cycler_instances);
    bail!("STOP");
    code_cyclers()?;
    code_structs()?;
    code_perception_databases_structs()?;
    Ok(())
}

pub fn write_token_stream(file_name: &str, token_stream: TokenStream) -> Result<()> {
    let file_path =
        PathBuf::from(var("OUT_DIR").wrap_err("failed to get environment variable OUT_DIR")?)
            .join(file_name);

    {
        let mut file = File::create(&file_path)
            .wrap_err_with(|| format!("failed create file {file_path:?}"))?;
        write!(file, "{token_stream}")
            .wrap_err_with(|| format!("failed to write to file {file_path:?}"))?;
    }

    let status = Command::new("rustfmt")
        .arg(file_path)
        .status()
        .wrap_err("failed to execute rustfmt")?;
    if !status.success() {
        bail!("rustfmt did not exit with success");
    }

    Ok(())
}

fn code_cyclers() -> Result<()> {
    //TODO rerun if changed
    let cycler_instances = CyclerInstances::try_from_crates_directory("..")
        .wrap_err("failed to get cycler instances from crates directory")?;
    let mut nodes = Nodes::try_from_crates_directory("..")
        .wrap_err("failed to get nodes from crates directory")?;
    nodes.sort().wrap_err("failed to sort nodes")?;
    let cycler_types = CyclerTypes::try_from_crates_directory("..")
        .wrap_err("failed to get perception cycler instances from crates directory")?;

    for node_names in nodes.cycler_modules_to_nodes.values() {
        let first_node_name = match node_names.first() {
            Some(first_node_name) => first_node_name,
            None => continue,
        };
        for field in nodes.nodes[first_node_name].contexts.cycle_context.iter() {
            match field {
                Field::HistoricInput { name, .. } => bail!(
                    "unexpected historic input for first node `{first_node_name}` in `{}` for `{name}` in cycle context",
                    nodes.nodes[first_node_name].cycler_module
                ),
                Field::Input { name, .. } => bail!(
                    "unexpected optional input for first node `{first_node_name}` in `{}` for `{name}` in cycle context",
                    nodes.nodes[first_node_name].cycler_module
                ),
                Field::PerceptionInput { name, .. } => bail!(
                    "unexpected perception input for first node `{first_node_name}` in `{}` for `{name}` in cycle context",
                    nodes.nodes[first_node_name].cycler_module
                ),
                Field::RequiredInput { name, .. } => bail!(
                    "unexpected required input for first node `{first_node_name}` in `{}` for `{name}` in cycle context",
                    nodes.nodes[first_node_name].cycler_module
                ),
                _ => {}
            }
        }
    }

    let cyclers = get_cyclers(&cycler_instances, &nodes, &cycler_types);

    let cyclers_token_stream = generate_cyclers(&cyclers).wrap_err("failed to generate cyclers")?;
    let runtime_token_stream = generate_run(&cyclers);

    write_token_stream(
        "cyclers.rs",
        quote! {
            #cyclers_token_stream
            #runtime_token_stream
        },
    )
    .wrap_err("failed to write cyclers")?;

    Ok(())
}

fn code_structs() -> Result<()> {
    let structs = Structs::try_from_crates_directory("..")
        .wrap_err("failed to get structs from crates directory")?;

    let configuration = match &structs.configuration {
        StructHierarchy::Struct { fields } => {
            let structs = struct_hierarchy_to_token_stream(
                "Configuration",
                fields,
                quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
            )
            .wrap_err("failed to generate struct `Configuration`")?;
            quote! {
                #structs
            }
        }
        StructHierarchy::Optional { .. } => bail!("unexpected optional variant as root-struct"),
        StructHierarchy::Field { .. } => bail!("unexpected field variant as root-struct"),
    };
    let cyclers = structs
        .cycler_structs
        .iter()
        .map(|(cycler_module, cycler_structs)| {
            let cycler_module_identifier = format_ident!("{}", cycler_module);
            let main_outputs = match &cycler_structs.main_outputs {
                StructHierarchy::Struct { fields } => struct_hierarchy_to_token_stream(
                    "MainOutputs",
                    fields,
                    quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
                )
                .wrap_err("failed to generate struct `MainOutputs`")?,
                StructHierarchy::Optional { .. } => {
                    bail!("unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("unexpected field variant as root-struct"),
            };
            let additional_outputs = match &cycler_structs.additional_outputs {
                StructHierarchy::Struct { fields } => struct_hierarchy_to_token_stream(
                    "AdditionalOutputs",
                    fields,
                    quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
                )
                .wrap_err("failed to generate struct `AdditionalOutputs`")?,
                StructHierarchy::Optional { .. } => {
                    bail!("unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("unexpected field variant as root-struct"),
            };
            let persistent_state = match &cycler_structs.persistent_state {
                StructHierarchy::Struct { fields } => struct_hierarchy_to_token_stream(
                    "PersistentState",
                    fields,
                    quote! { #[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, serialize_hierarchy::SerializeHierarchy)] },
                )
                .wrap_err("failed to generate struct `PersistentState`")?,
                StructHierarchy::Optional { .. } => {
                    bail!("unexpected optional variant as root-struct")
                }
                StructHierarchy::Field { .. } => bail!("unexpected field variant as root-struct"),
            };

            Ok(quote! {
                pub mod #cycler_module_identifier {
                    #main_outputs
                    #additional_outputs
                    #persistent_state
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let token_stream = quote! {
        #configuration
        #(#cyclers)*
    };

    write_token_stream("structs.rs", token_stream)
        .wrap_err("failed to write perception databases structs")?;

    Ok(())
}

fn struct_hierarchy_to_token_stream(
    struct_name: &str,
    fields: &BTreeMap<String, StructHierarchy>,
    derives: TokenStream,
) -> Result<TokenStream> {
    let struct_name_identifier = format_ident!("{}", struct_name);
    let struct_fields: Vec<_> = fields
        .iter()
        .map(|(name, struct_hierarchy)| {
            let name_identifier = format_ident!("{}", name);
            match struct_hierarchy {
                StructHierarchy::Struct { .. } => {
                    let struct_name_identifier =
                        format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
                    Ok(quote! { pub #name_identifier: #struct_name_identifier })
                }
                StructHierarchy::Optional { child } => match &**child {
                    StructHierarchy::Struct { .. } => {
                        let struct_name_identifier =
                            format_ident!("{}{}", struct_name, name.to_case(Case::Pascal));
                        Ok(quote! { pub #name_identifier: Option<#struct_name_identifier> })
                    }
                    StructHierarchy::Optional { .. } => {
                        bail!("unexpected optional in an optional struct")
                    }
                    StructHierarchy::Field { data_type } => {
                        Ok(quote! { pub #name_identifier: Option<#data_type> })
                    }
                },
                StructHierarchy::Field { data_type } => {
                    Ok(quote! { pub #name_identifier: #data_type })
                }
            }
        })
        .collect::<Result<_, _>>()
        .wrap_err("failed to generate struct fields")?;
    let child_structs: Vec<_> = fields
        .iter()
        .map(|(name, struct_hierarchy)| match struct_hierarchy {
            StructHierarchy::Struct { fields } => {
                let struct_name = format!("{}{}", struct_name, name.to_case(Case::Pascal));
                struct_hierarchy_to_token_stream(&struct_name, fields, derives.clone())
                    .wrap_err_with(|| format!("failed to generate struct `{struct_name}`"))
            }
            StructHierarchy::Optional { child } => match &**child {
                StructHierarchy::Struct { fields } => {
                    let struct_name = format!("{}{}", struct_name, name.to_case(Case::Pascal));
                    struct_hierarchy_to_token_stream(&struct_name, fields, derives.clone())
                        .wrap_err_with(|| format!("failed to generate struct `{struct_name}`"))
                }
                StructHierarchy::Optional { .. } => {
                    bail!("unexpected optional in an optional struct")
                }
                StructHierarchy::Field { .. } => Ok(Default::default()),
            },
            StructHierarchy::Field { .. } => Ok(Default::default()),
        })
        .collect::<Result<_, _>>()
        .wrap_err("failed to generate child structs")?;

    Ok(quote! {
        #derives
        pub struct #struct_name_identifier {
            #(#struct_fields,)*
        }
        #(#child_structs)*
    })
}

fn code_perception_databases_structs() -> Result<()> {
    let cycler_instances = CyclerInstances::try_from_crates_directory("..")
        .wrap_err("failed to get cycler instances from crates directory")?;
    let cycler_types = CyclerTypes::try_from_crates_directory("..")
        .wrap_err("failed to get perception cycler instances from crates directory")?;

    let updates_fields = cycler_instances.instances_to_modules.iter().filter_map(|(instance_name, module_name)| {
        match cycler_types.cycler_modules_to_cycler_types[module_name] {
            CyclerType::Perception => {
                let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
                let module_name_identifier = format_ident!("{}", module_name);
                Some(quote! { pub #field_name_identifier: Update<structs::#module_name_identifier::MainOutputs> })
            },
            CyclerType::RealTime => None,
        }
    });
    let timestamp_array_items = cycler_instances
        .instances_to_modules
        .iter()
        .filter_map(|(instance_name, module_name)| {
            match cycler_types.cycler_modules_to_cycler_types[module_name] {
                CyclerType::Perception => {
                    let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
                    Some(quote! { self.#field_name_identifier.first_timestamp_of_non_finalized_database })
                },
                CyclerType::RealTime => None,
            }
        });
    let push_loops =
        cycler_instances
            .instances_to_modules
            .iter()
            .filter_map(|(instance_name, module_name)| {
                match cycler_types.cycler_modules_to_cycler_types[module_name] {
                    CyclerType::Perception => {
                        let field_name_identifier =
                            format_ident!("{}", instance_name.to_case(Case::Snake));
                        Some(quote! {
                            for timestamped_database in self.#field_name_identifier.items {
                                databases
                                    .get_mut(&timestamped_database.timestamp)
                                    .unwrap()
                                    .#field_name_identifier
                                    .push(timestamped_database.data);
                            }
                        })
                    }
                    CyclerType::RealTime => None,
                }
            });
    let databases_fields = cycler_instances.instances_to_modules.iter().filter_map(|(instance_name, module_name)| {
        match cycler_types.cycler_modules_to_cycler_types[module_name] {
            CyclerType::Perception => {
                let field_name_identifier = format_ident!("{}", instance_name.to_case(Case::Snake));
                let module_name_identifier = format_ident!("{}", module_name);
                Some(quote! { pub #field_name_identifier: Vec<structs::#module_name_identifier::MainOutputs> })
            },
            CyclerType::RealTime => None,
        }
    });

    write_token_stream(
        "perception_databases_structs.rs",
        quote! {
            pub struct Updates {
                #(#updates_fields,)*
            }

            impl framework::Updates<Databases> for Updates {
                fn first_timestamp_of_temporary_databases(&self) -> Option<std::time::SystemTime> {
                    [
                        #(#timestamp_array_items,)*
                    ]
                    .iter()
                    .copied()
                    .flatten()
                    .min()
                }

                fn push_to_databases(self, databases: &mut std::collections::BTreeMap<std::time::SystemTime, Databases>) {
                    #(#push_loops)*
                }
            }

            pub struct Update<MainOutputs> {
                pub items: Vec<framework::Item<MainOutputs>>,
                pub first_timestamp_of_non_finalized_database: Option<std::time::SystemTime>,
            }

            #[derive(Default)]
            pub struct Databases {
                #(#databases_fields,)*
            }
        },
    )
    .wrap_err("failed to write perception databases structs")?;

    Ok(())
}
