/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::class::data_models::fields::Fields;
use crate::util::{bail, KvParser};
use crate::ParseResult;
use std::cmp::Ordering;

/// Points to index of a given group name in [Fields.groups](field@Fields::groups).
///
/// Two fields with the same GroupIdentifier belong to the same group.
pub type GroupIdentifier = usize;

pub struct FieldGroup {
    pub group_name_index: Option<GroupIdentifier>,
    pub subgroup_name_index: Option<GroupIdentifier>,
}

impl FieldGroup {
    fn parse_group(
        expr: &'static str,
        parser: &mut KvParser,
        groups: &mut Vec<String>,
    ) -> ParseResult<Option<GroupIdentifier>> {
        let Some(group) = parser.handle_string(expr)? else {
            return Ok(None);
        };

        if let Some(group_index) = groups
            .iter()
            .position(|existing_group| existing_group == &group)
        {
            Ok(Some(group_index))
        } else {
            groups.push(group);
            Ok(Some(groups.len() - 1))
        }
    }

    pub(crate) fn new_from_kv(
        parser: &mut KvParser,
        groups: &mut Vec<String>,
    ) -> ParseResult<Self> {
        let (group_name_index, subgroup_name_index) = (
            Self::parse_group("group", parser, groups)?,
            Self::parse_group("subgroup", parser, groups)?,
        );

        // Declaring only a subgroup for given property – with no group at all – is totally valid in Godot.
        // Unfortunately it leads to a lot of very janky and not too ideal behaviours
        // So it is better to treat it as a user error.
        if subgroup_name_index.is_some() && group_name_index.is_none() {
            return bail!(parser.span(), "Subgroups without groups are not supported.");
        }

        Ok(Self {
            group_name_index,
            subgroup_name_index,
        })
    }
}

/// Remove surrounding quotes to display declared "group name" in editor as `group name` instead of `"group name"`.
/// Should be called after parsing all the fields to avoid unnecessary operations.
pub(crate) fn format_groups(groups: Vec<String>) -> Vec<String> {
    groups
        .into_iter()
        .map(|g| g.trim_matches('"').to_string())
        .collect()
}

// ----------------------------------------------------------------------------------------------------------------------------------------------
// Ordering

pub(crate) struct ExportGroupOrdering {
    /// Allows to identify given export group.
    /// `None` for root.
    identifier: Option<GroupIdentifier>,
    /// Contains subgroups of given ordering (subgroups for groups, subgroups&groups for root).
    /// Ones parsed first have higher priority, i.e. are displayed as the first.
    subgroups: Vec<ExportGroupOrdering>,
}

impl ExportGroupOrdering {
    /// Creates root which holds all the groups&subgroups.
    /// Should be called only once in a given context.
    fn root() -> Self {
        Self {
            identifier: None,
            subgroups: Vec::new(),
        }
    }

    /// Represents individual group & its subgroups.
    fn child(identifier: GroupIdentifier) -> Self {
        Self {
            identifier: Some(identifier),
            subgroups: Vec::new(),
        }
    }

    /// Returns registered group index. Registers given group if not present.
    fn group_index(&mut self, identifier: &GroupIdentifier) -> usize {
        self.subgroups
            .iter()
            // Will never fail – non-root orderings must have an identifier.
            .position(|sub| identifier == sub.identifier.as_ref().expect("Tried to parse an undefined export group. This is a bug, please report it."))
            .unwrap_or_else(|| {
                // Register new subgroup.
                self.subgroups.push(ExportGroupOrdering::child(*identifier));
                self.subgroups.len() - 1
            })
    }
}

// Note: GDExtension doesn't support categories for some reason(s?).
// It probably expects us to use inheritance instead?
enum OrderingStage {
    Group,
    SubGroup,
}

// It is recursive but max recursion depth is 2 (root -> group -> subgroup) so it's fine.
fn compare_by_group_and_declaration_order(
    field_a: &FieldGroup,
    field_b: &FieldGroup,
    ordering: &mut ExportGroupOrdering,
    stage: OrderingStage,
) -> Ordering {
    let (lhs, rhs, next_stage) = match stage {
        OrderingStage::Group => (
            &field_a.group_name_index,
            &field_b.group_name_index,
            Some(OrderingStage::SubGroup),
        ),
        OrderingStage::SubGroup => (
            &field_a.subgroup_name_index,
            &field_b.subgroup_name_index,
            None,
        ),
    };

    match (lhs, rhs) {
        // Ungrouped fields or fields with subgroup only always have higher priority (i.e. are displayed on top).
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,

        // Same group/subgroup.
        (Some(group_a), Some(group_b)) => {
            if group_a == group_b {
                let Some(next_stage) = next_stage else {
                    return Ordering::Equal;
                };

                let next_ordering_position = ordering.group_index(group_a);

                // Fields belong to the same group – check the subgroup.
                compare_by_group_and_declaration_order(
                    field_a,
                    field_b,
                    &mut ordering.subgroups[next_ordering_position],
                    next_stage,
                )
            } else {
                // Parsed earlier => greater priority.
                let (priority_a, priority_b) = (
                    usize::MAX - ordering.group_index(group_a),
                    usize::MAX - ordering.group_index(group_b),
                );
                priority_b.cmp(&priority_a)
            }
        }

        (None, None) => {
            // Fields don't belong to any subgroup nor group.
            let Some(next_stage) = next_stage else {
                return Ordering::Equal;
            };

            compare_by_group_and_declaration_order(field_a, field_b, ordering, next_stage)
        }
    }
}

/// Sorts fields by their group and subgroup association.
///
/// Fields without group nor subgroup are first.
/// Fields with subgroup only come in next, in order of their declaration on the class struct.
/// Finally fields with groups are displayed – firstly ones without subgroups followed by
/// fields with given group & subgroup (in the same order as above).
///
/// Group membership for properties in Godot is based on the order of their registration.
/// All the properties belong to group or subgroup registered beforehand – thus the need to sort them.
pub(crate) fn sort_fields_by_group(fields: &mut Fields) {
    let mut initial_ordering = ExportGroupOrdering::root();

    // `sort_by` instead of `sort_unstable_by` to preserve original order of declaration.
    // Which is not guaranteed by the way albeit worked reliably so far.
    fields.all_fields.sort_by(|a, b| {
        let (group_a, group_b) = match (&a.group, &b.group) {
            (Some(a), Some(b)) => (a, b),
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            // We don't care about ordering of fields without a `#[export]`.
            _ => return Ordering::Equal,
        };

        compare_by_group_and_declaration_order(
            group_a,
            group_b,
            &mut initial_ordering,
            OrderingStage::Group,
        )
    });
}
