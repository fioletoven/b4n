use kube::api::DynamicObject;
use std::{collections::HashSet, rc::Rc};

use crate::{
    kubernetes::resources::{CrdColumns, ResourceData},
    ui::lists::{Column, Header, NAMESPACE},
};

/// Returns [`ResourceData`] for the custom resource.
pub fn data(crd: &CrdColumns, object: &DynamicObject) -> ResourceData {
    let is_terminating = object.metadata.deletion_timestamp.is_some();

    ResourceData {
        extra_values: Box::default(),
        is_job: false,
        is_completed: false,
        is_ready: !is_terminating,
        is_terminating,
    }
}

/// Returns [`Header`] for the custom resource.
pub fn header(crd: &CrdColumns) -> Header {
    let columns = if let Some(columns) = &crd.columns {
        columns.iter().map(Column::from).collect()
    } else {
        Vec::new()
    };
    let symbols = get_sort_symbols(&columns);

    Header::from(
        NAMESPACE.clone(),
        Some(columns.into_boxed_slice()),
        Rc::from(symbols.into_boxed_slice()),
    )
}

fn get_sort_symbols(columns: &Vec<Column>) -> Vec<char> {
    let mut already_taken = HashSet::with_capacity(columns.len() + 2);
    already_taken.insert('N');
    already_taken.insert('A');

    let mut symbols = Vec::with_capacity(columns.len() + 3);
    symbols.push(' ');
    symbols.push('N');

    for column in columns {
        let mut found = false;
        for ch in column.name.chars() {
            if !already_taken.contains(&ch) {
                symbols.push(ch);
                already_taken.insert(ch);

                found = true;
                break;
            }
        }

        if !found {
            symbols.push(' ');
        }
    }

    symbols.push('A');
    symbols
}
