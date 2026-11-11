use oxc_ast::ast::*;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::matcher::var_function_or_declaration;

use crate::array_rotator::rotator_unit;
use crate::proof::RotatorSite;
use crate::stamp::{decoder_index, is_machinery_name, string_array_name, string_array_unit};

pub struct MachineryDepth {
    names: Option<FxHashSet<String>>,
    rotators: FxHashSet<usize>,
    rotator_sites: Vec<(usize, RotatorSite)>,
    depth: usize,
    decoder_units: FxHashMap<usize, usize>,
    owners: Vec<Option<usize>>,
}

impl MachineryDepth {
    pub fn every_unit() -> Self {
        Self::owned_by(FxHashMap::default())
    }

    pub fn owned_by(decoder_units: FxHashMap<usize, usize>) -> Self {
        Self {
            names: None,
            rotators: FxHashSet::default(),
            rotator_sites: Vec::new(),
            depth: 0,
            decoder_units,
            owners: Vec::new(),
        }
    }

    pub fn one_unit(names: FxHashSet<String>) -> Self {
        Self {
            names: Some(names),
            ..Self::every_unit()
        }
    }

    fn unit_of(&self, name: &str) -> Option<usize> {
        string_array_unit(name).or_else(|| {
            decoder_index(name).and_then(|index| self.decoder_units.get(&index).copied())
        })
    }

    pub fn exempts(&self, name: &str) -> bool {
        if self.depth == 0 {
            return false;
        }
        let Some(unit) = self.unit_of(name) else {
            return false;
        };
        self.owners.iter().all(|owner| *owner == Some(unit))
    }

    pub fn names(&self) -> Option<&FxHashSet<String>> {
        self.names.as_ref()
    }

    pub fn covers(&self, name: &str) -> bool {
        match &self.names {
            Some(names) => names.contains(name),
            None => is_machinery_name(name),
        }
    }

    pub fn take_rotator_sites(&mut self) -> Vec<(usize, RotatorSite)> {
        std::mem::take(&mut self.rotator_sites)
    }

    pub fn inside(&self) -> bool {
        self.depth > 0
    }

    pub fn enter(&mut self, stmt: &Statement<'_>) -> bool {
        let Some(owner) = self.is_unit(stmt) else {
            return false;
        };
        self.depth += 1;
        self.owners.push(owner);
        true
    }

    pub fn leave(&mut self, entered: bool) {
        if entered {
            self.depth -= 1;
            self.owners.pop();
        }
    }

    fn is_unit(&mut self, stmt: &Statement<'_>) -> Option<Option<usize>> {
        if let Some(decl) = var_function_or_declaration(stmt) {
            if self.covers(decl.name) {
                return Some(self.unit_of(decl.name));
            }
        }
        let unit = rotator_unit(stmt)?;
        if !self.covers(&string_array_name(unit)) {
            return None;
        }
        self.rotator_sites.push((unit, RotatorSite::of(stmt)));
        self.rotators.insert(unit).then_some(Some(unit))
    }
}
