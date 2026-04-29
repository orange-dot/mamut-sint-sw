use mamut_params::MacroId;
use mamut_patch::{MacroDefaults, MacroResponseCurve, PatchFileV1};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MacroState {
    pub gravitacija: f32,
    pub bloom: f32,
    pub heat: f32,
    pub ruin: f32,
    pub swarm: f32,
}

impl MacroState {
    pub fn from_defaults(defaults: &MacroDefaults) -> Self {
        Self {
            gravitacija: defaults.gravitacija,
            bloom: defaults.bloom,
            heat: defaults.heat,
            ruin: defaults.ruin,
            swarm: defaults.swarm,
        }
    }

    pub fn clamped(self) -> Self {
        Self {
            gravitacija: self.gravitacija.clamp(0.0, 1.0),
            bloom: self.bloom.clamp(0.0, 1.0),
            heat: self.heat.clamp(0.0, 1.0),
            ruin: self.ruin.clamp(0.0, 1.0),
            swarm: self.swarm.clamp(0.0, 1.0),
        }
    }

    pub fn get(self, id: MacroId) -> f32 {
        match id {
            MacroId::Gravitacija => self.gravitacija,
            MacroId::Bloom => self.bloom,
            MacroId::Heat => self.heat,
            MacroId::Ruin => self.ruin,
            MacroId::Swarm => self.swarm,
        }
    }

    pub fn set(&mut self, id: MacroId, value: f32) {
        match id {
            MacroId::Gravitacija => self.gravitacija = value,
            MacroId::Bloom => self.bloom = value,
            MacroId::Heat => self.heat = value,
            MacroId::Ruin => self.ruin = value,
            MacroId::Swarm => self.swarm = value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IdentityState {
    pub horizont_open: f32,
    pub horizont_air: f32,
    pub horizont_span: f32,
    pub pec_mass: f32,
    pub pec_heat: f32,
    pub pec_pressure: f32,
    pub baklja_ready: f32,
    pub baklja_edge: f32,
    pub baklja_sync_bias: f32,
    pub grav_pull: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DerivedState {
    pub mass: f32,
    pub strain: f32,
    pub headroom: f32,
    pub body_focus: f32,
    pub rupture_threshold: f32,
    pub rupture_response: f32,
    pub spatial_dispersion: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ResolvedIdentityFrame {
    pub raw_macros: MacroState,
    pub shaped_macros: MacroState,
    pub identity: IdentityState,
    pub derived: DerivedState,
}

pub fn resolve_identity(patch: &PatchFileV1, live_macros: &MacroState) -> ResolvedIdentityFrame {
    let raw_macros = live_macros.clamped();
    let shaped_macros = shape_macros(patch, raw_macros);
    let identity = resolve_identity_state(patch, shaped_macros);
    let derived = resolve_derived_state(patch, identity, shaped_macros);

    ResolvedIdentityFrame {
        raw_macros,
        shaped_macros,
        identity,
        derived,
    }
}

fn shape_macros(patch: &PatchFileV1, raw_macros: MacroState) -> MacroState {
    let mut shaped_macros = MacroState::default();

    for macro_id in MacroId::ALL {
        let raw_value = raw_macros.get(macro_id);
        let response = patch.macro_response.spec(macro_id);
        let scaled_value = (raw_value * response.sensitivity).clamp(0.0, 1.0);
        let curved_value = match response.curve {
            MacroResponseCurve::Linear => scaled_value,
            MacroResponseCurve::SoftPlus => {
                mix(scaled_value, scaled_value.sqrt(), response.softness)
            }
            MacroResponseCurve::LateRise => {
                mix(scaled_value, scaled_value.powf(1.6), response.softness)
            }
            MacroResponseCurve::EarlyRise => mix(
                scaled_value,
                1.0 - (1.0 - scaled_value).powf(1.6),
                response.softness,
            ),
        };
        shaped_macros.set(macro_id, curved_value.clamp(0.0, 1.0));
    }

    shaped_macros
}

fn resolve_identity_state(patch: &PatchFileV1, macros: MacroState) -> IdentityState {
    let biases = patch.identity_bias;
    let late_gravity = late_stage(macros.gravitacija, 0.68);
    let extreme_swarm = late_stage(macros.swarm, 0.82);

    IdentityState {
        horizont_open: clamp01(
            macros.bloom * 0.70 + (1.0 - macros.gravitacija) * 0.25 + biases.horizont_bias * 0.35
                - macros.ruin * 0.10
                - macros.heat * 0.05,
        ),
        horizont_air: clamp01(
            macros.bloom * 0.78 + (1.0 - macros.heat) * 0.08 + biases.horizont_bias * 0.22
                - late_gravity * 0.18,
        ),
        horizont_span: clamp01(
            macros.bloom * 0.58 + macros.swarm * 0.32 + biases.horizont_bias * 0.16
                - macros.gravitacija * 0.16,
        ),
        pec_mass: clamp01(
            macros.heat * 0.60
                + macros.gravitacija * 0.25
                + biases.pec_bias * 0.40
                + macros.swarm * 0.05
                - macros.bloom * 0.05,
        ),
        pec_heat: clamp01(macros.heat * 0.80 + macros.gravitacija * 0.12 + biases.pec_bias * 0.25),
        pec_pressure: clamp01(
            macros.gravitacija * 0.58
                + macros.heat * 0.24
                + macros.ruin * 0.08
                + biases.gravitacija_pressure_bias * 0.35,
        ),
        baklja_ready: clamp01(
            macros.ruin * 0.56
                + macros.gravitacija * 0.28
                + biases.baklja_bias * 0.42
                + extreme_swarm * 0.08,
        ),
        baklja_edge: clamp01(macros.ruin * 0.68 + late_gravity * 0.40 + biases.baklja_bias * 0.18),
        baklja_sync_bias: clamp01(
            macros.ruin * 0.72 + macros.heat * 0.06 + biases.baklja_bias * 0.20,
        ),
        grav_pull: macros.gravitacija,
    }
}

fn resolve_derived_state(
    patch: &PatchFileV1,
    identity: IdentityState,
    macros: MacroState,
) -> DerivedState {
    let biases = patch.identity_bias;

    let mass =
        clamp01(identity.pec_mass * 0.70 + identity.pec_heat * 0.20 + identity.grav_pull * 0.10);
    let strain = clamp01(
        identity.baklja_edge * 0.40 + identity.pec_pressure * 0.38 + identity.grav_pull * 0.22,
    );
    let headroom = clamp01(
        0.78 + identity.horizont_air * 0.18 + macros.bloom * 0.08
            - identity.grav_pull * 0.46
            - macros.heat * 0.14
            - identity.baklja_ready * 0.08,
    );
    let body_focus = clamp01(
        identity.pec_mass * 0.50 + identity.grav_pull * 0.28 + biases.body_focus_bias * 0.35
            - macros.bloom * 0.18,
    );
    let rupture_threshold = clamp01(
        0.78 - identity.baklja_ready * 0.22 - identity.grav_pull * 0.16
            + biases.rupture_threshold_bias * 0.22,
    );
    let rupture_response =
        clamp01(identity.baklja_edge * 0.55 + strain * 0.25 + macros.ruin * 0.20);
    let spatial_dispersion =
        clamp01(identity.horizont_span * 0.55 + macros.swarm * 0.33 - identity.grav_pull * 0.10);

    DerivedState {
        mass,
        strain,
        headroom,
        body_focus,
        rupture_threshold,
        rupture_response,
        spatial_dispersion,
    }
}

fn late_stage(value: f32, threshold: f32) -> f32 {
    if value <= threshold {
        0.0
    } else {
        ((value - threshold) / (1.0 - threshold)).clamp(0.0, 1.0)
    }
}

fn mix(base: f32, alternate: f32, amount: f32) -> f32 {
    base + (alternate - base) * amount.clamp(0.0, 1.0)
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_patch::load_patch_toml;

    const MOLTEN_HORIZON: &str = include_str!("../../../patches/factory/molten-horizon.toml");

    fn fixture() -> PatchFileV1 {
        load_patch_toml(MOLTEN_HORIZON).expect("fixture must parse")
    }

    #[test]
    fn identity_resolution_is_deterministic() {
        let patch = fixture();
        let macros = MacroState::from_defaults(&patch.macros);

        let left = resolve_identity(&patch, &macros);
        let right = resolve_identity(&patch, &macros);

        assert_eq!(left, right);
    }

    #[test]
    fn bloom_expands_horizont() {
        let patch = fixture();
        let base = resolve_identity(
            &patch,
            &MacroState {
                bloom: 0.10,
                ..MacroState::default()
            },
        );
        let bloom = resolve_identity(
            &patch,
            &MacroState {
                bloom: 0.90,
                ..MacroState::default()
            },
        );

        assert!(bloom.identity.horizont_open > base.identity.horizont_open);
        assert!(bloom.identity.horizont_air > base.identity.horizont_air);
    }

    #[test]
    fn heat_strengthens_pec() {
        let patch = fixture();
        let cold = resolve_identity(
            &patch,
            &MacroState {
                heat: 0.10,
                ..MacroState::default()
            },
        );
        let hot = resolve_identity(
            &patch,
            &MacroState {
                heat: 0.90,
                ..MacroState::default()
            },
        );

        assert!(hot.identity.pec_mass > cold.identity.pec_mass);
        assert!(hot.identity.pec_heat > cold.identity.pec_heat);
    }

    #[test]
    fn gravitacija_reduces_headroom_and_arms_baklja() {
        let patch = fixture();
        let loose = resolve_identity(
            &patch,
            &MacroState {
                gravitacija: 0.10,
                ..MacroState::default()
            },
        );
        let heavy = resolve_identity(
            &patch,
            &MacroState {
                gravitacija: 0.90,
                ..MacroState::default()
            },
        );

        assert!(heavy.derived.headroom < loose.derived.headroom);
        assert!(heavy.identity.baklja_ready > loose.identity.baklja_ready);
    }
}
