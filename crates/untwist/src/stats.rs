use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodeStats {
    pub steps: u64,
    pub peak_unit_steps: u64,
    pub units: u32,
    pub peak_call_depth: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollisionStats {
    pub bound_in_scope: u32,
    pub free_reference: u32,
    pub label: u32,
    pub excluded: u32,
}

impl CollisionStats {
    pub fn total(&self) -> u64 {
        [
            self.bound_in_scope,
            self.free_reference,
            self.label,
            self.excluded,
        ]
        .iter()
        .map(|&count| u64::from(count))
        .sum()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RunStats {
    pub decode: DecodeStats,
    pub collisions: CollisionStats,
}

impl RunStats {
    pub(crate) fn taken() -> RunStats {
        let decode = untwist_eval::stats::take();
        let collisions = untwist_rename::collisions::take();
        RunStats {
            decode: DecodeStats {
                steps: decode.steps,
                peak_unit_steps: decode.peak_unit_steps,
                units: decode.units,
                peak_call_depth: decode.peak_call_depth,
            },
            collisions: CollisionStats {
                bound_in_scope: collisions.bound_in_scope,
                free_reference: collisions.free_reference,
                label: collisions.label,
                excluded: collisions.excluded,
            },
        }
    }

    pub(crate) fn merged(self, other: RunStats) -> RunStats {
        let (a, b) = (self.decode, other.decode);
        let (c, d) = (self.collisions, other.collisions);
        RunStats {
            decode: DecodeStats {
                steps: a.steps.saturating_add(b.steps),
                peak_unit_steps: a.peak_unit_steps.max(b.peak_unit_steps),
                units: a.units.saturating_add(b.units),
                peak_call_depth: a.peak_call_depth.max(b.peak_call_depth),
            },
            collisions: CollisionStats {
                bound_in_scope: c.bound_in_scope.saturating_add(d.bound_in_scope),
                free_reference: c.free_reference.saturating_add(d.free_reference),
                label: c.label.saturating_add(d.label),
                excluded: c.excluded.saturating_add(d.excluded),
            },
        }
    }
}
