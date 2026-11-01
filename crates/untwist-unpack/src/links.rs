use oxc_ast::ast::Program;

use crate::parcel::links::ParcelView;
use crate::turbopack::links::{Group, TurboView, NAMESPACE};

#[derive(Default)]
pub struct Links {
    pub parcel: Option<ParcelView>,
    pub turbopack: Option<TurboView>,
}

pub struct Member {
    pub namespace: String,
    pub inputs: Vec<usize>,
    pub turbopack: Option<Group>,
}

#[derive(Default)]
pub struct Groups {
    pub views: Vec<Links>,
    pub members: Vec<Member>,
    skipped: Vec<usize>,
}

impl Groups {
    pub fn skipped(&self, input: usize) -> bool {
        self.skipped.contains(&input)
    }
}

pub fn groups(programs: &[Program<'_>]) -> Groups {
    if programs.len() < 2 {
        return Groups::default();
    }
    let mut groups = Groups {
        views: programs.iter().map(|_| Links::default()).collect(),
        ..Groups::default()
    };
    for group in crate::parcel::links::groups(programs) {
        let Some(namespace) = group.first().map(|(_, view)| view.namespace.clone()) else {
            continue;
        };
        let inputs = group.iter().map(|(input, _)| *input).collect();
        for (input, view) in group {
            groups.views[input].parcel = Some(view);
        }
        groups.members.push(Member {
            namespace,
            inputs,
            turbopack: None,
        });
    }
    if let Some(group) = crate::turbopack::links::group(programs) {
        for &input in &group.chunks {
            groups.views[input].turbopack = Some(group.view.clone());
        }
        groups.skipped.extend([group.loader, group.runtime]);
        groups.members.push(Member {
            namespace: NAMESPACE.to_string(),
            inputs: group.chunks.clone(),
            turbopack: Some(group),
        });
    }
    groups
}
