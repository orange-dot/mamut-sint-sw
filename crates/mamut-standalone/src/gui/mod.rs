use std::{env, path::PathBuf, time::Instant};

use anyhow::{Context, Result, anyhow};
use eframe::{NativeOptions, egui};
use mamut_engine::{BcsLayerMode, BcsScenario, EngineSnapshot, GfmLayerMode, GfmTerrainSnapshot16};
use mamut_params::{MacroId, ParamId, ParamSection, ParamSpec, ParamUnit, all_params, param_spec};
use mamut_runtime::*;

const ENGINE_SCOPE_BUFFER_FRAMES: usize = 8_192;
const ENGINE_SCOPE_DEFAULT_WINDOW_FRAMES: usize = 2_048;

mod style;
pub(crate) use style::*;

mod pc4_display;
pub(crate) use pc4_display::*;

mod binding_widgets;
pub(crate) use binding_widgets::*;

mod sound_lab_display;
pub(crate) use sound_lab_display::*;

mod param_values;

mod events;
pub(crate) use events::*;

mod window;
pub(crate) use window::*;

mod app;
pub(crate) use app::*;

mod app_methods;
mod debug_methods;
mod engine_methods;
mod inspect_methods;
mod live_methods;
mod pc4_methods;
mod sound_lab_methods;
