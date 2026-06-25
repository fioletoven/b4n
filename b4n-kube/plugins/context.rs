use std::borrow::Cow;

use crate::{Kind, Namespace, ResourceRef};

#[cfg(test)]
#[path = "./context.tests.rs"]
mod context_tests;

/// Execution context for a plugin.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct PluginContext {
    pub context: String,
    pub kind: Kind,
    pub namespace: Namespace,
    pub resources: Vec<ResourceRef>,
    pub columns: Vec<String>,
    pub values: Vec<Vec<String>>,
}

impl PluginContext {
    /// Resolves command argument for all rows or a specified one.
    pub fn resolve_arg(&self, arg: &str, row_index: Option<usize>) -> String {
        let mut result = String::with_capacity(arg.len());
        let mut remaining = arg;

        while let Some(dollar_pos) = remaining.find('$') {
            result.push_str(&remaining[..dollar_pos]);
            remaining = &remaining[dollar_pos..];

            if let Some((replacement, skip)) = self.try_resolve_placeholder(remaining, row_index) {
                result.push_str(&replacement);
                remaining = &remaining[skip..];
            } else {
                result.push('$');
                remaining = &remaining[1..];
            }
        }

        result.push_str(remaining);
        result
    }

    fn try_resolve_placeholder<'a>(&'a self, s: &str, row_index: Option<usize>) -> Option<(Cow<'a, str>, usize)> {
        type PlaceholderResolver = (&'static str, fn(&PluginContext) -> &str);
        let simple: &[PlaceholderResolver] = &[
            ("$CONTEXT", |ctx: &PluginContext| ctx.context.as_str()),
            ("$PLURAL", |ctx: &PluginContext| ctx.kind.name()),
            ("$GROUP", |ctx: &PluginContext| ctx.kind.group()),
            ("$VERSION", |ctx: &PluginContext| ctx.kind.version()),
            ("$NAMESPACE", |ctx: &PluginContext| ctx.namespace.as_str()),
        ];

        for (prefix, resolver) in simple {
            if s.starts_with(prefix) {
                return Some((Cow::Borrowed(resolver(self)), prefix.len()));
            }
        }

        if s.starts_with("$COUNT") {
            return Some((Cow::Owned(self.resources.len().to_string()), 6));
        }

        if s.starts_with("$COL[") {
            let close_pos = s.find(']')?;
            let col_name = &s["$COL[".len()..close_pos];
            let value = self.resolve_col(col_name, row_index);

            return Some((value, close_pos + 1));
        }

        if s.starts_with("$RES[") {
            let close_pos = s.find(']')?;
            let field_name = &s["$RES[".len()..close_pos];
            let value = self.resolve_res(field_name, row_index);

            return Some((value, close_pos + 1));
        }

        None
    }

    fn resolve_col<'a>(&'a self, col_name: &str, row_index: Option<usize>) -> Cow<'a, str> {
        let Some(col_index) = self.columns.iter().position(|c| c.eq_ignore_ascii_case(col_name)) else {
            return Cow::Borrowed("");
        };

        if let Some(row_index) = row_index {
            let value = self
                .values
                .get(row_index)
                .and_then(|row| row.get(col_index))
                .map_or("", String::as_str);
            Cow::Borrowed(value)
        } else {
            let joined = self
                .values
                .iter()
                .filter_map(|row| row.get(col_index).map(String::as_str))
                .collect::<Vec<_>>()
                .join(",");
            Cow::Owned(joined)
        }
    }

    fn resolve_res<'a>(&'a self, field_name: &str, row_index: Option<usize>) -> Cow<'a, str> {
        let extract: fn(&ResourceRef) -> Option<&str> = match field_name.to_ascii_uppercase().as_str() {
            "NAME" => |r| r.name.as_deref(),
            "NAMESPACE" => |r| r.namespace.as_option(),
            "UID" => |r| r.uid.as_deref(),
            "CONTAINER" => |r| r.container.as_deref(),
            _ => return Cow::Borrowed(""),
        };

        if let Some(index) = row_index {
            let value = self.resources.get(index).and_then(|r| extract(r)).unwrap_or("");
            Cow::Borrowed(value)
        } else {
            let joined = self.resources.iter().filter_map(extract).collect::<Vec<_>>().join(",");
            Cow::Owned(joined)
        }
    }
}
