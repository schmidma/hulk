use std::collections::BTreeMap;

use crate::configuration::CyclerConfiguration;

#[derive(Debug)]
pub struct CyclerInstances {
    pub instance_to_module: BTreeMap<String, String>,
    //pub module_to_instance: BTreeMap<String, Vec<String>>,
}

impl CyclerInstances {
    pub fn from_configuration<'a>(
        values: impl IntoIterator<Item = &'a CyclerConfiguration>,
    ) -> Self {
        let mut instances_to_modules = BTreeMap::new();
        let mut modules_to_instances: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for value in values.into_iter() {
            match &value.instances {
                Some(instances) => {
                    for instance in instances {
                        let instance = format!("{}{}", value.name, instance);
                        instances_to_modules.insert(instance.clone(), value.module.clone());
                        modules_to_instances
                            .entry(value.module.clone())
                            .or_default()
                            .push(instance);
                    }
                }
                None => {
                    instances_to_modules.insert(value.name.clone(), value.module.clone());
                    modules_to_instances
                        .entry(value.module.clone())
                        .or_default()
                        .push(value.name.clone());
                }
            }
        }
        Self {
            instance_to_module: instances_to_modules,
            module_to_instance: modules_to_instances,
        }
    }
}
